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
