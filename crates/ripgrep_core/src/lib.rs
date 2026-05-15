//! ripgrep_core — Swift-facing wrapper around ripgrep's reusable crates.
//!
//! Public surface is generated via UniFFI; see `lib.rs` `uniffi::setup_scaffolding!()`.

mod error;
mod options;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_compiles() {
        assert_eq!(2 + 2, 4);
    }
}

#[cfg(test)]
mod error_tests {
    use super::error::RipgrepError;

    #[test]
    fn error_messages_render() {
        let e = RipgrepError::InvalidPattern("[".into());
        assert!(format!("{e}").contains("invalid regex"));
        assert!(format!("{e}").contains("["));
    }
}

#[cfg(test)]
mod options_tests {
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
            timeout_ms: None,
        };
        assert_eq!(r.pattern, "todo");
    }
}

mod cancel;

mod sink;

mod search;

#[cfg(test)]
mod search_matcher_tests {
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
            timeout_ms: None,
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
}

#[cfg(test)]
mod sink_tests {
    use super::sink::ChannelSink;
    use crossbeam_channel::unbounded;
    use grep_regex::RegexMatcher;
    use grep_searcher::SearcherBuilder;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn sink_emits_match_per_line() {
        let (tx, rx) = unbounded();
        let counter = Arc::new(AtomicUsize::new(0));
        let m = RegexMatcher::new("foo").unwrap();
        let mut sink = ChannelSink::new(
            "/tmp/test.txt".into(),
            tx,
            Arc::new(crate::cancel::CancelToken::new(None)),
            Arc::clone(&counter),
            m.clone(),
            0,
        );
        let body = b"foo\nbar\nfoo\n";
        SearcherBuilder::new()
            .build()
            .search_slice(&m, body, &mut sink)
            .unwrap();
        drop(sink);
        let collected: Vec<_> = rx.iter().collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].line_number, 1);
        assert_eq!(collected[1].line_number, 3);
    }
}

#[cfg(test)]
mod cancel_tests {
    use super::cancel::CancelToken;
    use std::sync::Arc;
    use std::time::Duration;
    use std::thread;

    #[test]
    fn fresh_token_not_cancelled() {
        let t = CancelToken::new(None);
        assert!(!t.is_cancelled());
    }

    #[test]
    fn explicit_cancel_trips_token() {
        let t = Arc::new(CancelToken::new(None));
        let t2 = Arc::clone(&t);
        thread::spawn(move || t2.cancel());
        thread::sleep(Duration::from_millis(20));
        assert!(t.is_cancelled());
    }

    #[test]
    fn deadline_trips_token_after_elapse() {
        let t = CancelToken::new(Some(10));
        thread::sleep(Duration::from_millis(40));
        assert!(t.is_cancelled());
    }

    #[test]
    fn no_deadline_means_never_auto_cancel() {
        let t = CancelToken::new(None);
        thread::sleep(Duration::from_millis(20));
        assert!(!t.is_cancelled());
    }
}

#[cfg(test)]
mod walker_tests {
    use super::options::SearchRequest;
    use super::search::build_walker;
    use std::path::Path;

    fn req() -> SearchRequest {
        SearchRequest {
            pattern: "TODO".into(),
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
            timeout_ms: None,
        }
    }

    fn collect_paths(r: SearchRequest) -> Vec<String> {
        let walker = build_walker(&r).unwrap().build();
        walker
            .filter_map(Result::ok)
            .filter(|d| d.file_type().map_or(false, |t| t.is_file()))
            .map(|d| d.into_path().to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn respects_gitignore() {
        let paths = collect_paths(req());
        assert!(paths.iter().any(|p| p.ends_with("included.txt")));
        assert!(!paths.iter().any(|p| p.ends_with("ignored.txt")));
        assert!(!paths.iter().any(|p| p.contains("/target/")));
    }

    #[test]
    fn skips_hidden_by_default() {
        let paths = collect_paths(req());
        assert!(!paths.iter().any(|p| p.ends_with(".hidden.txt")));
    }

    #[test]
    fn includes_hidden_when_flag_set() {
        let mut r = req();
        r.include_hidden = true;
        let paths = collect_paths(r);
        assert!(paths.iter().any(|p| p.ends_with(".hidden.txt")));
    }

    #[test]
    fn no_ignore_includes_gitignored() {
        let mut r = req();
        r.respect_gitignore = false;
        let paths = collect_paths(r);
        assert!(paths.iter().any(|p| p.ends_with("ignored.txt")));
    }

    #[test]
    fn glob_include_filters_files() {
        let mut r = req();
        r.include_globs = vec!["*.swift".into()];
        let paths = collect_paths(r);
        assert!(paths.iter().all(|p| p.ends_with(".swift")));
    }

    #[test]
    fn glob_exclude_drops_files() {
        let mut r = req();
        r.exclude_globs = vec!["*.swift".into()];
        let paths = collect_paths(r);
        assert!(!paths.iter().any(|p| p.ends_with(".swift")));
    }

    #[test]
    fn file_type_filter_swift_only() {
        let mut r = req();
        r.file_types = vec!["swift".into()];
        let paths = collect_paths(r);
        assert!(paths.iter().all(|p| p.ends_with(".swift")));
    }

    #[test]
    fn nonexistent_path_errors() {
        let mut r = req();
        r.paths = vec!["/no/such/path".into()];
        let result = build_walker(&r);
        assert!(matches!(result, Err(crate::error::RipgrepError::PathNotFound(_))));
    }
}
