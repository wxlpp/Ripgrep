#[test]
fn force_panic_is_translated_to_internal_panic() {
    let err = super::search::force_panic_for_test().unwrap_err();
    match err {
        crate::error::RipgrepError::InternalPanic(msg) => {
            assert!(msg.contains("intentional"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
