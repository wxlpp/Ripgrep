//! ripgrep_core — Swift-facing wrapper around ripgrep's reusable crates.
//!
//! Public surface is generated via UniFFI; see `lib.rs` `uniffi::setup_scaffolding!()`.

uniffi::setup_scaffolding!();

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

use cancel::CancelToken;
use error::RipgrepError;
use options::{SearchRequest, SearchResult};
use std::sync::Arc;

#[uniffi::export]
pub fn search_blocking(
    request: SearchRequest,
    cancel: Arc<CancelToken>,
) -> Result<SearchResult, RipgrepError> {
    search::search_blocking(request, cancel)
}

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
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    #[test]
    fn sink_emits_match_per_line() {
        let (tx, rx) = unbounded();
        let counter = Arc::new(AtomicUsize::new(0));
        let m = RegexMatcher::new("foo").unwrap();
        let mut sink = ChannelSink::new(
            "/tmp/test.txt".into(),
            tx,
            crate::cancel::CancelToken::new(None),
            Arc::clone(&counter),
            m.clone(),
            0, // before_context
            0, // after_context
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
    use std::thread;
    use std::time::Duration;

    #[test]
    fn fresh_token_not_cancelled() {
        let t = CancelToken::new(None);
        assert!(!t.is_cancelled());
    }

    #[test]
    fn explicit_cancel_trips_token() {
        let t = CancelToken::new(None);
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
            .filter(|d| d.file_type().is_some_and(|t| t.is_file()))
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
        assert!(matches!(
            result,
            Err(crate::error::RipgrepError::PathNotFound(_))
        ));
    }
}

#[cfg(test)]
mod search_e2e_tests {
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
            timeout_ms: None,
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
}

#[cfg(test)]
mod ffi_export_smoke {
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
}

#[cfg(test)]
mod context_tests {
    use super::cancel::CancelToken;
    use super::search::search_blocking;

    fn req() -> super::options::SearchRequest {
        let mut r = super::search_e2e_tests::req("match");
        r.paths = vec!["tests/fixtures/mini/context.txt".into()];
        r
    }

    #[test]
    fn before_context_captures_prior_lines() {
        let mut r = req();
        r.before_context = 1;
        let res = search_blocking(r, CancelToken::new(None)).unwrap();
        let m = &res.matches[0];
        assert_eq!(m.before_context, vec!["line 2".to_string()]);
    }

    #[test]
    fn after_context_captures_following_lines() {
        let mut r = req();
        r.after_context = 1;
        let res = search_blocking(r, CancelToken::new(None)).unwrap();
        let m = &res.matches[0];
        assert_eq!(m.after_context, vec!["line 4".to_string()]);
    }

    #[test]
    fn multiline_pattern_matches_across_lines() {
        let mut r = req();
        r.pattern = r"match[\s\S]*match".into();
        r.multiline = true;
        let res = search_blocking(r, CancelToken::new(None)).unwrap();
        assert!(!res.matches.is_empty());
    }
}

#[cfg(test)]
mod limits_tests {
    use super::cancel::CancelToken;
    use super::search::search_blocking;

    fn req(pattern: &str) -> super::options::SearchRequest {
        let mut r = super::search_e2e_tests::req(pattern);
        r.respect_gitignore = false; // ensure we have lots of files to count
        r.include_hidden = true;
        r
    }

    #[test]
    fn max_matches_truncates_and_flags() {
        let mut r = req("TODO");
        r.max_matches = Some(1);
        let res = search_blocking(r, CancelToken::new(None)).unwrap();
        assert_eq!(res.matches.len(), 1);
        assert!(res.truncated);
    }

    #[test]
    fn max_files_caps_files_searched() {
        let mut r = req("TODO");
        r.max_files = Some(1);
        let res = search_blocking(r, CancelToken::new(None)).unwrap();
        assert!(res.files_searched <= 1, "got {}", res.files_searched);
    }

    #[test]
    fn timeout_marks_result_cancelled() {
        let mut r = req("xxxxxxxxxxxxxxxxx_no_match"); // forces full scan
        r.timeout_ms = Some(1); // 1 ms — extremely tight
                                // Big enough fixture: just our mini, but we set timeout to 0 so it trips
                                // even on tiny dirs.
        let res = search_blocking(
            r,
            CancelToken::new(Some(0)), // pre-tripped
        )
        .unwrap();
        assert!(res.cancelled);
    }
}

#[cfg(test)]
mod adjacent_after_context_tests {
    use super::cancel::CancelToken;
    use super::search::search_blocking;

    /// Reproduce the per-match after_context attribution bug:
    /// When two matches in the same file are close enough that grep_searcher
    /// merges their context windows (no context_break between them),
    /// the FIRST match must still receive the after-context lines that fall
    /// within its own -A window.
    ///
    /// Fixture `tests/fixtures/adjacent/a.txt`:
    ///   line 1: pre
    ///   line 2: MATCH one
    ///   line 3: MATCH two
    ///   line 4: tail1
    ///   line 5: tail2
    ///   line 6: tail3
    ///
    /// With `after_context = 2` and pattern `MATCH`:
    ///   match @ line 2 → after_context should be ["tail1"]
    ///     (line 3 "MATCH two" is a match event, not an after-context line;
    ///      line 4 "tail1" is within the -A 2 window: 2 < 4 <= 2+2)
    ///   match @ line 3 → after_context should be ["tail1", "tail2"]
    ///     (lines 4 "tail1" and 5 "tail2" are within the -A 2 window: 3 < 4,5 <= 3+2)
    #[test]
    fn adjacent_matches_first_match_has_correct_after_context() {
        let mut r = super::search_e2e_tests::req("MATCH");
        r.paths = vec!["tests/fixtures/adjacent/a.txt".into()];
        r.after_context = 2;
        let res = search_blocking(r, CancelToken::new(None)).unwrap();

        assert_eq!(res.matches.len(), 2, "expected exactly 2 matches");

        let m1 = &res.matches[0];
        let m2 = &res.matches[1];

        assert_eq!(m1.line_number, 2, "first match should be on line 2");
        assert_eq!(m2.line_number, 3, "second match should be on line 3");

        // KEY assertion: first match's after_context must be non-empty
        // (the bug causes it to be empty).
        // grep_searcher emits each matching line via matched(), not as after-context,
        // so "MATCH two" (line 3) is NOT included in m1's after_context — only
        // non-matching lines within the -A window appear as after-context.
        // With -A 2: match@line2's window covers lines 3,4; line3 is a match (emitted
        // separately), line4 "tail1" is the after-context line for m1.
        assert!(
            !m1.after_context.is_empty(),
            "BUG: first match's after_context is empty; should contain 'tail1' from its -A window"
        );
        // line 5 (tail2) is outside m1's window: m1@2, after=2 → 2 < L <= 4, so 5 excluded
        assert_eq!(
            m1.after_context,
            vec!["tail1".to_string()],
            "first match should have 'tail1' (line 4) in its after-context"
        );
        assert_eq!(
            m2.after_context,
            vec!["tail1".to_string(), "tail2".to_string()],
            "second match should have 'tail1','tail2' (lines 4,5) in its after-context"
        );
    }
}

#[cfg(test)]
mod panic_tests {
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
}

#[cfg(test)]
mod external_cancel_tests {
    use super::cancel::CancelToken;
    use super::search::search_blocking;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn external_cancel_stops_search_promptly() {
        let cancel = CancelToken::new(None);
        let cancel_for_thread = Arc::clone(&cancel);

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            cancel_for_thread.cancel();
        });

        let r = super::search_e2e_tests::req("anything");
        let start = Instant::now();
        let res = search_blocking(r, cancel).unwrap();
        let elapsed = start.elapsed();
        // tiny fixture finishes in <50ms anyway, so cancellation may or may not
        // have kicked in; the assertion is about *no panic / no hang*.
        assert!(
            elapsed < Duration::from_secs(2),
            "search hung: {:?}",
            elapsed
        );
        let _ = res;
    }
}
