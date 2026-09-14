#![no_main]

#[path = "common.rs"]
mod common;

use libfuzzer_sys::fuzz_target;
use ohmygrep_core::SearchRequest;

fuzz_target!(|input: (String, bool, bool, bool)| {
    let (pattern, case_insensitive, smart_case, multiline) = input;
    let request = SearchRequest {
        pattern,
        paths: vec![common::corpus_dir().to_string()],
        case_insensitive,
        smart_case,
        multiline,
        max_matches: Some(100),
        max_columns: Some(64),
        ..Default::default()
    };
    common::check(common::search(request), Some(100));
});
