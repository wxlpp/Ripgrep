use super::cancel::CancelToken;
use super::options::SearchRequest;
use super::search::search_blocking;

pub(super) fn req(pattern: &str) -> SearchRequest {
    SearchRequest {
        pattern: pattern.into(),
        paths: vec!["tests/fixtures/mini".into()],
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
fn finds_matches_in_fixture() {
    let r = search_blocking(req("TODO"), CancelToken::new(None)).unwrap();
    let texts: Vec<_> = r.matches.iter().map(|m| m.line.as_str()).collect();
    assert!(texts.iter().any(|l| l.contains("search me TODO")));
    assert!(texts.iter().any(|l| l.contains("// TODO: handle case")));
    assert!(!texts.iter().any(|l| l.contains("should not be searched")));
}

#[test]
fn results_sorted_deterministically() {
    let r = search_blocking(req("TODO"), CancelToken::new(None)).unwrap();
    let mut prev: Option<(&str, u64)> = None;
    for m in &r.matches {
        let key = (m.path.as_str(), m.line_number);
        if let Some(p) = prev {
            assert!(p <= key, "results not sorted: {p:?} then {key:?}");
        }
        prev = Some(key);
    }
}

#[test]
fn empty_result_when_no_match() {
    let r = search_blocking(req("ZZZZ_NOPE"), CancelToken::new(None)).unwrap();
    assert_eq!(r.matches.len(), 0);
    assert!(!r.truncated);
    assert!(!r.cancelled);
}
