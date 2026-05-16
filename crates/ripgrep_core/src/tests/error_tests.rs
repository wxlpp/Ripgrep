use super::error::RipgrepError;

#[test]
fn error_messages_render() {
    let e = RipgrepError::InvalidPattern("[".into());
    assert!(format!("{e}").contains("invalid regex"));
    assert!(format!("{e}").contains("["));
}
