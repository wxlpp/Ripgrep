use super::options::*;

#[test]
fn search_request_default_constructable() {
    let r = SearchRequest {
        pattern: "todo".into(),
        paths: vec![".".into()],
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
    };
    assert_eq!(r.pattern, "todo");
}
