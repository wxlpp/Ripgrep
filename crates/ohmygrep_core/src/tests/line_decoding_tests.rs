use super::cancel::CancelToken;
use super::search::search_blocking;
use super::support::{req, TempDir};

#[test]
fn crlf_match_and_context_lines_have_no_trailing_cr() {
    let dir = TempDir::new("crlf");
    let file = dir.write("f.txt", b"alpha\r\nbeta\r\ngamma\r\n");
    let mut r = req("beta", &file);
    r.before_context = 1;
    r.after_context = 1;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1);
    let m = &res.matches[0];
    assert_eq!(m.line, "beta");
    assert_eq!(m.before_context, vec!["alpha".to_string()]);
    assert_eq!(m.after_context, vec!["gamma".to_string()]);
}

#[test]
fn line_without_terminator_is_kept() {
    let dir = TempDir::new("noeol");
    let file = dir.write("f.txt", b"hello");
    let res = search_blocking(req("hello", &file), CancelToken::new(None)).unwrap();
    assert_eq!(res.matches[0].line, "hello");
}

#[test]
fn bare_cr_inside_line_is_content() {
    let dir = TempDir::new("cr");
    let file = dir.write("f.txt", b"a\rb\n");
    let res = search_blocking(req("a", &file), CancelToken::new(None)).unwrap();
    assert_eq!(res.matches[0].line, "a\rb");
}

#[test]
fn multiline_crlf_keeps_inner_terminators() {
    let dir = TempDir::new("mlcrlf");
    let file = dir.write("f.txt", b"alpha\r\nbeta\r\ngamma\r\n");
    let mut r = req(r"alpha\r\nbeta", &file);
    r.multiline = true;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1);
    assert_eq!(res.matches[0].line, "alpha\r\nbeta");
}

#[test]
fn multiline_match_keeps_trailing_blank_line() {
    let dir = TempDir::new("mlblank");
    let file = dir.write("f.txt", b"x\nfoo\n\nbar\n");
    let mut r = req(r"foo\n\n", &file);
    r.multiline = true;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches[0].line, "foo\n");
}

#[test]
fn submatches_stay_within_line() {
    let dir = TempDir::new("submatch");
    let file = dir.write("f.txt", b"TODO\r\nTODO later\n");
    let res = search_blocking(req(r"TODO\s*", &file), CancelToken::new(None)).unwrap();
    for m in &res.matches {
        for s in &m.submatches {
            assert!(
                s.end as usize <= m.line.len(),
                "submatch {s:?} exceeds line {:?} ({} bytes)",
                m.line,
                m.line.len()
            );
        }
    }
    assert_eq!(res.matches[0].submatches[0].end, 4);
    assert_eq!(res.matches[1].submatches[0].end, 5);
}

#[test]
fn end_anchor_matches_before_terminator() {
    let dir = TempDir::new("anchor");
    let file = dir.write("f.txt", b"a TODO\n");
    let res = search_blocking(req(r"TODO$", &file), CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1);
    assert_eq!(res.matches[0].submatches.len(), 1);
}

#[test]
fn binary_files_are_skipped_by_default() {
    let dir = TempDir::new("bin");
    dir.write("text.txt", b"HIT text\n");
    dir.write("blob.dat", b"x\0y HIT\n");
    let res = search_blocking(req("HIT", &dir.path()), CancelToken::new(None)).unwrap();
    let files: Vec<_> = res.matches.iter().map(|m| m.path.clone()).collect();
    assert_eq!(files.len(), 1, "got {files:?}");
    assert!(files[0].ends_with("text.txt"));
}

#[test]
fn search_binary_includes_binary_files() {
    let dir = TempDir::new("bin_on");
    dir.write("text.txt", b"HIT text\n");
    dir.write("blob.dat", b"x\0y HIT\n");
    let mut r = req("HIT", &dir.path());
    r.search_binary = true;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 2);
}
