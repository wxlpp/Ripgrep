use super::options::*;

#[test]
fn search_request_default_constructable() {
    let r = SearchRequest {
        pattern: "todo".into(),
        paths: vec![".".into()],
        respect_gitignore: true,
        ..Default::default()
    };
    assert_eq!(r.pattern, "todo");
}
