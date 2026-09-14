use super::error::OhMyGrepError;

#[test]
fn error_messages_render() {
    let e = OhMyGrepError::InvalidPattern("[".into());
    assert!(format!("{e}").contains("invalid regex"));
    assert!(format!("{e}").contains("["));
}
