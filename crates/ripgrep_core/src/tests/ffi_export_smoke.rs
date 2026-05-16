use super::cancel::CancelToken;

#[test]
fn ffi_search_blocking_returns_ok_with_matches() {
    let r = super::search_e2e_tests::req("TODO");
    let res = crate::search_blocking(r, CancelToken::new(None)).unwrap();
    assert!(
        !res.matches.is_empty(),
        "expected at least one TODO match in fixture"
    );
}
