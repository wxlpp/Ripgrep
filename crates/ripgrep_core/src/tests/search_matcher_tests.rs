use super::options::SearchRequest;
use super::search::build_matcher;

fn req(pattern: &str) -> SearchRequest {
    SearchRequest {
        pattern: pattern.into(),
        paths: vec![],
        case_insensitive: false,
        smart_case: false,
        multiline: false,
        include_globs: vec![],
        exclude_globs: vec![],
        file_types: vec![],
        respect_gitignore: true,
        include_hidden: false,
        before_context: 0,
        after_context: 0,
        max_matches: None,
        max_files: None,
        max_file_size_bytes: None,
    }
}

#[test]
fn case_sensitive_by_default() {
    let m = build_matcher(&req("Foo")).unwrap();
    use grep_matcher::Matcher;
    assert!(m.find(b"Foo").unwrap().is_some());
    assert!(m.find(b"foo").unwrap().is_none());
}

#[test]
fn case_insensitive_matches_any_case() {
    let mut r = req("Foo");
    r.case_insensitive = true;
    let m = build_matcher(&r).unwrap();
    use grep_matcher::Matcher;
    assert!(m.find(b"FOO").unwrap().is_some());
}

#[test]
fn smart_case_off_for_uppercase_pattern() {
    let mut r = req("Foo");
    r.smart_case = true;
    let m = build_matcher(&r).unwrap();
    use grep_matcher::Matcher;
    assert!(m.find(b"foo").unwrap().is_none());
}

#[test]
fn smart_case_insensitive_for_lowercase_pattern() {
    let mut r = req("foo");
    r.smart_case = true;
    let m = build_matcher(&r).unwrap();
    use grep_matcher::Matcher;
    assert!(m.find(b"FOO").unwrap().is_some());
}

#[test]
fn invalid_pattern_returns_error() {
    let r = req("[");
    let err = build_matcher(&r).unwrap_err();
    assert!(matches!(err, crate::error::RipgrepError::InvalidPattern(_)));
}
