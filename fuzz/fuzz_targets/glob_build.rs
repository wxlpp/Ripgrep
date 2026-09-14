#![no_main]

#[path = "common.rs"]
mod common;

use libfuzzer_sys::fuzz_target;
use ohmygrep_core::SearchRequest;

fuzz_target!(
    |input: (Vec<String>, Vec<String>, Vec<String>, bool, bool)| {
        let (include_globs, exclude_globs, file_types, respect_gitignore, require_git) = input;
        let request = SearchRequest {
            pattern: "HIT".into(),
            paths: vec![common::corpus_dir().to_string()],
            include_globs,
            exclude_globs,
            file_types,
            respect_gitignore,
            require_git,
            include_hidden: true,
            max_matches: Some(100),
            ..Default::default()
        };
        common::check(common::search(request), Some(100));
    }
);
