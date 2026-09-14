use super::cancel::CancelToken;
use super::error::OhMyGrepError;
use super::options::SearchResult;
use super::search::search_blocking;
use super::support::{req, TempDir};
use super::warnings::MAX_WARNINGS;
use std::os::unix::fs::PermissionsExt;

fn files(res: &SearchResult) -> Vec<String> {
    let mut names: Vec<String> = res
        .matches
        .iter()
        .map(|m| m.path.rsplit('/').next().unwrap().to_string())
        .collect();
    names.sort();
    names.dedup();
    names
}

fn ignore_tree() -> TempDir {
    let dir = TempDir::new("ignore");
    dir.write(".gitignore", b"git_ignored.txt\n");
    dir.write(".ignore", b"dot_ignored.txt\n");
    dir.write(".rgignore", b"rg_ignored.txt\n");
    for name in [
        "kept.txt",
        "git_ignored.txt",
        "dot_ignored.txt",
        "rg_ignored.txt",
    ] {
        dir.write(name, b"HIT\n");
    }
    dir
}

fn search_tree(dir: &TempDir, respect: bool, require_git: bool) -> Vec<String> {
    let mut r = req("HIT", &dir.path());
    r.respect_gitignore = respect;
    r.require_git = require_git;
    files(&search_blocking(r, CancelToken::new(None)).unwrap())
}

#[test]
fn gitignore_outside_git_repo_needs_require_git_false() {
    let dir = ignore_tree();
    assert_eq!(
        search_tree(&dir, true, true),
        vec!["git_ignored.txt", "kept.txt"]
    );
    assert_eq!(search_tree(&dir, true, false), vec!["kept.txt"]);
}

#[test]
fn no_ignore_includes_everything() {
    let dir = ignore_tree();
    assert_eq!(
        search_tree(&dir, false, false),
        vec![
            "dot_ignored.txt",
            "git_ignored.txt",
            "kept.txt",
            "rg_ignored.txt"
        ]
    );
}

#[test]
fn empty_paths_are_invalid_arguments() {
    let mut r = req("HIT", "unused");
    r.paths.clear();
    let err = search_blocking(r, CancelToken::new(None)).unwrap_err();
    assert!(matches!(err, OhMyGrepError::InvalidArguments(_)), "{err:?}");
}

#[test]
fn bad_glob_is_invalid_arguments() {
    let dir = TempDir::new("glob");
    let mut r = req("HIT", &dir.path());
    r.include_globs = vec!["a{".into()];
    let err = search_blocking(r, CancelToken::new(None)).unwrap_err();
    assert!(matches!(err, OhMyGrepError::InvalidArguments(_)), "{err:?}");
}

/// Mode 000 is still readable by root (common in CI containers).
fn lock(path: &str) -> bool {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
    std::fs::read(path).is_err()
}

#[test]
fn unreadable_file_becomes_warning() {
    let dir = TempDir::new("unreadable");
    dir.write("ok.txt", b"HIT\n");
    let locked = dir.write("locked.txt", b"HIT\n");
    if !lock(&locked) {
        eprintln!("skipped: running as root");
        return;
    }
    let res = search_blocking(req("HIT", &dir.path()), CancelToken::new(None)).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(files(&res), vec!["ok.txt"]);
    assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
    assert_eq!(res.warnings[0].path, locked);
    assert!(
        res.warnings[0]
            .message
            .to_lowercase()
            .contains("permission"),
        "{:?}",
        res.warnings[0]
    );
}

#[test]
fn warnings_are_capped_with_overflow_note() {
    let dir = TempDir::new("cap");
    let extra = 5;
    let mut locked = Vec::new();
    for i in 0..(MAX_WARNINGS + extra) {
        let path = dir.write(&format!("f{i}.txt"), b"HIT\n");
        if !lock(&path) {
            eprintln!("skipped: running as root");
            return;
        }
        locked.push(path);
    }
    let res = search_blocking(req("HIT", &dir.path()), CancelToken::new(None)).unwrap();
    for path in &locked {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    assert_eq!(res.warnings.len(), MAX_WARNINGS + 1);
    let last = res.warnings.last().unwrap();
    assert_eq!(
        (last.path.as_str(), last.message.as_str()),
        ("", format!("{extra} more warnings omitted").as_str())
    );
    let paths: Vec<_> = res.warnings[..MAX_WARNINGS]
        .iter()
        .map(|w| &w.path)
        .collect();
    assert!(
        paths.windows(2).all(|w| w[0] <= w[1]),
        "not sorted: {paths:?}"
    );
}

#[test]
fn walked_binary_file_with_match_before_nul_keeps_match_and_warns() {
    let dir = TempDir::new("late_nul");
    let mut content = b"HIT early\n".to_vec();
    content.extend(std::iter::repeat_n(b'.', 300_000));
    content.extend_from_slice(b"\nx\0y\nHIT late\n");
    let file = dir.write("late.dat", &content);
    let res = search_blocking(req("HIT", &dir.path()), CancelToken::new(None)).unwrap();
    let lines: Vec<_> = res.matches.iter().map(|m| m.line.as_str()).collect();
    assert_eq!(lines, vec!["HIT early"]);
    assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
    assert_eq!(res.warnings[0].path, file);
    assert!(res.warnings[0]
        .message
        .starts_with("stopped searching binary file after match"));
}

#[test]
fn walked_binary_file_without_prior_match_is_silent() {
    let dir = TempDir::new("early_nul");
    dir.write("blob.dat", b"x\0y HIT\n");
    let res = search_blocking(req("HIT", &dir.path()), CancelToken::new(None)).unwrap();
    assert!(res.matches.is_empty());
    assert!(res.warnings.is_empty(), "{:?}", res.warnings);
}

#[test]
fn named_binary_file_reports_binary_match() {
    let dir = TempDir::new("named_bin");
    let file = dir.write("blob.dat", b"x\0y HIT\n");
    let res = search_blocking(req("HIT", &file), CancelToken::new(None)).unwrap();
    assert!(res.matches.is_empty(), "{:?}", res.matches);
    assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
    assert!(res.warnings[0]
        .message
        .starts_with("binary file matches (found \"\\0\" byte around offset"));
}

#[test]
fn named_binary_file_without_match_is_silent() {
    let dir = TempDir::new("named_bin_none");
    let file = dir.write("blob.dat", b"x\0y\n");
    let res = search_blocking(req("HIT", &file), CancelToken::new(None)).unwrap();
    assert!(res.matches.is_empty());
    assert!(res.warnings.is_empty(), "{:?}", res.warnings);
}

#[test]
fn invalid_ignore_file_glob_becomes_warning() {
    let dir = TempDir::new("badignore");
    dir.write(".ignore", b"a{\n");
    dir.write("f.txt", b"HIT\n");
    let mut r = req("HIT", &dir.path());
    r.respect_gitignore = true;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1);
    assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
    assert!(
        res.warnings[0].path.ends_with(".ignore"),
        "{:?}",
        res.warnings[0]
    );
    assert!(
        res.warnings[0].message.contains("a{"),
        "{:?}",
        res.warnings[0]
    );
}

/// Symlinks are not followed (rg's default without -L), so a self-referencing link is inert.
#[test]
fn symlinks_are_not_followed() {
    let dir = TempDir::new("symlink");
    dir.write("a.txt", b"HIT\n");
    std::os::unix::fs::symlink(dir.path(), format!("{}/loop", dir.path())).unwrap();
    let res = search_blocking(req("HIT", &dir.path()), CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1, "{:?}", res.matches);
}
