/// TDD for v0.4-CRLF-1: CRLF line terminator leaves a stray `\r` in emitted
/// lines when the fix is absent.
///
/// Non-vacuity: with the OLD `trim_end_matches('\n')` (no `.strip_suffix('\r')`),
/// a CRLF file's lines arrive as `b"foo\r\n"` → `"foo\r"` after trimming only
/// `\n`. The assertions below fail on that stray `\r`. Reverting the fix in
/// sink.rs makes every assertion in `crlf_match_line_has_no_trailing_cr` and
/// `crlf_context_lines_have_no_trailing_cr` fail.
use std::io::Write as _;
use std::path::PathBuf;

use super::cancel::CancelToken;
use super::options::SearchRequest;
use super::search::search_blocking;

/// RAII guard: deletes the temp file on drop (even on panic).
struct TempFile(PathBuf);

impl TempFile {
    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Write `content` (arbitrary bytes) to a unique temp file and return a guard
/// that deletes the file on drop.
fn write_temp_bytes(name: &str, content: &[u8]) -> TempFile {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ripgrep_core_crlf_test_{name}_{}.txt",
        std::process::id()
    ));
    let mut f = std::fs::File::create(&path).expect("create temp file");
    f.write_all(content).expect("write temp file");
    TempFile(path)
}

fn base_req(path: &str, pattern: &str) -> SearchRequest {
    SearchRequest {
        pattern: pattern.into(),
        paths: vec![path.into()],
        case_insensitive: false,
        smart_case: false,
        multiline: false,
        include_globs: vec![],
        exclude_globs: vec![],
        file_types: vec![],
        respect_gitignore: false,
        include_hidden: true,
        before_context: 0,
        after_context: 0,
        max_matches: None,
        max_files: None,
        max_file_size_bytes: None,
    }
}

// (a) CRLF match line: emitted `line` must not end with `\r`.
#[test]
fn crlf_match_line_has_no_trailing_cr() {
    // Non-vacuity: OLD code emits "alpha\r" — assert_eq fails, test reports the bug.
    let _tmp = write_temp_bytes("match", b"alpha\r\nbeta\r\ngamma\r\n");
    let r = search_blocking(base_req(_tmp.path(), "beta"), CancelToken::new(None)).unwrap();
    assert_eq!(r.matches.len(), 1);
    let line = &r.matches[0].line;
    assert!(
        !line.ends_with('\r'),
        "CRLF match line has trailing \\r: {line:?}"
    );
    assert_eq!(line, "beta", "expected clean match line, got: {line:?}");
}

// (b) CRLF before- and after-context lines must not end with `\r`.
#[test]
fn crlf_context_lines_have_no_trailing_cr() {
    // Non-vacuity: OLD code emits "alpha\r" / "gamma\r" in context.
    let _tmp = write_temp_bytes("ctx", b"alpha\r\nbeta\r\ngamma\r\n");
    let mut r = base_req(_tmp.path(), "beta");
    r.before_context = 1;
    r.after_context = 1;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1);
    let m = &res.matches[0];

    let before = &m.before_context;
    assert_eq!(before.len(), 1);
    assert!(
        !before[0].ends_with('\r'),
        "CRLF before-context has trailing \\r: {:?}",
        before[0]
    );
    assert_eq!(before[0], "alpha");

    let after = &m.after_context;
    assert_eq!(after.len(), 1);
    assert!(
        !after[0].ends_with('\r'),
        "CRLF after-context has trailing \\r: {:?}",
        after[0]
    );
    assert_eq!(after[0], "gamma");
}

// (c) LF regression: LF-only files are byte-identical to before.
#[test]
fn lf_file_unchanged() {
    let _tmp = write_temp_bytes("lf", b"alpha\nbeta\ngamma\n");
    let mut r = base_req(_tmp.path(), "beta");
    r.before_context = 1;
    r.after_context = 1;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1);
    let m = &res.matches[0];
    assert_eq!(m.line, "beta");
    assert_eq!(m.before_context, vec!["alpha".to_string()]);
    assert_eq!(m.after_context, vec!["gamma".to_string()]);
}

// (c cont.) No-trailing-newline regression: bare line must still decode cleanly.
#[test]
fn no_trailing_newline_unchanged() {
    let _tmp = write_temp_bytes("noterminator", b"hello");
    let r = search_blocking(base_req(_tmp.path(), "hello"), CancelToken::new(None)).unwrap();
    assert_eq!(r.matches.len(), 1);
    assert_eq!(r.matches[0].line, "hello");
}

// (d) Multiline CRLF: internal `\r\n` preserved; only trailing terminator stripped.
//
// With multiline=true, `grep_searcher` delivers the entire match as one buffer.
// The buffer for a match spanning "alpha\r\nbeta\r\n" is those exact bytes;
// only the final `\n` (and one preceding `\r` when present) should be stripped.
// Internal `\r\n` sequences are content and must be preserved.
#[test]
fn crlf_multiline_internal_crlf_preserved() {
    // File: three CRLF lines. Pattern spans lines 1-2.
    let _tmp = write_temp_bytes("ml", b"alpha\r\nbeta\r\ngamma\r\n");
    let mut r = base_req(_tmp.path(), r"alpha\r\nbeta");
    r.multiline = true;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1, "expected one multiline match");
    let line = &res.matches[0].line;
    // Internal \r\n is content — it must survive.
    assert!(
        line.contains("\r\n"),
        "multiline match: internal \\r\\n was stripped (it should be preserved): {line:?}"
    );
    // The trailing terminator must be gone.
    assert!(
        !line.ends_with('\r'),
        "multiline match still ends with \\r: {line:?}"
    );
}
