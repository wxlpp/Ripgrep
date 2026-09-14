use super::options::SearchRequest;
use super::search::build_walker;

fn req() -> SearchRequest {
    SearchRequest {
        pattern: "TODO".into(),
        paths: vec!["tests/fixtures/mini".into()],
        respect_gitignore: true,
        // The fixtures live in this repo; do not depend on that for gitignore.
        require_git: false,
        ..Default::default()
    }
}

fn collect_paths(r: SearchRequest) -> Vec<String> {
    let walker = build_walker(&r).unwrap().build();
    walker
        .filter_map(Result::ok)
        .filter(|d| d.file_type().is_some_and(|t| t.is_file()))
        .map(|d| d.into_path().to_string_lossy().to_string())
        .collect()
}

#[test]
fn respects_gitignore() {
    let paths = collect_paths(req());
    assert!(paths.iter().any(|p| p.ends_with("included.txt")));
    assert!(!paths.iter().any(|p| p.ends_with("ignored.txt")));
    assert!(!paths.iter().any(|p| p.contains("/target/")));
}

#[test]
fn skips_hidden_by_default() {
    let paths = collect_paths(req());
    assert!(!paths.iter().any(|p| p.ends_with(".hidden.txt")));
}

#[test]
fn includes_hidden_when_flag_set() {
    let mut r = req();
    r.include_hidden = true;
    let paths = collect_paths(r);
    assert!(paths.iter().any(|p| p.ends_with(".hidden.txt")));
}

#[test]
fn no_ignore_includes_gitignored() {
    let mut r = req();
    r.respect_gitignore = false;
    let paths = collect_paths(r);
    assert!(paths.iter().any(|p| p.ends_with("ignored.txt")));
}

#[test]
fn glob_include_filters_files() {
    let mut r = req();
    r.include_globs = vec!["*.swift".into()];
    let paths = collect_paths(r);
    assert!(paths.iter().all(|p| p.ends_with(".swift")));
}

#[test]
fn glob_exclude_drops_files() {
    let mut r = req();
    r.exclude_globs = vec!["*.swift".into()];
    let paths = collect_paths(r);
    assert!(!paths.iter().any(|p| p.ends_with(".swift")));
}

#[test]
fn file_type_filter_swift_only() {
    let mut r = req();
    r.file_types = vec!["swift".into()];
    let paths = collect_paths(r);
    assert!(paths.iter().all(|p| p.ends_with(".swift")));
}

#[test]
fn nonexistent_path_errors() {
    let mut r = req();
    r.paths = vec!["/no/such/path".into()];
    let result = build_walker(&r);
    assert!(matches!(
        result,
        Err(crate::error::OhMyGrepError::PathNotFound(_))
    ));
}
