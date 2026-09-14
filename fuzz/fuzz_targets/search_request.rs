#![no_main]

#[path = "common.rs"]
mod common;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use ohmygrep_core::{CancelToken, SearchRequest, SearchSession};

#[derive(Arbitrary, Debug)]
struct Input {
    pattern: String,
    case_insensitive: bool,
    smart_case: bool,
    multiline: bool,
    respect_gitignore: bool,
    require_git: bool,
    include_hidden: bool,
    search_binary: bool,
    before_context: u8,
    after_context: u8,
    max_matches: Option<u16>,
    max_files: Option<u8>,
    max_file_size_bytes: Option<u32>,
    max_columns: Option<u16>,
    batch: u8,
    streamed: bool,
}

fuzz_target!(|input: Input| {
    let max_matches = input.max_matches.map(u32::from);
    let request = SearchRequest {
        pattern: input.pattern,
        paths: vec![common::corpus_dir().to_string()],
        case_insensitive: input.case_insensitive,
        smart_case: input.smart_case,
        multiline: input.multiline,
        respect_gitignore: input.respect_gitignore,
        require_git: input.require_git,
        include_hidden: input.include_hidden,
        search_binary: input.search_binary,
        before_context: u32::from(input.before_context % 8),
        after_context: u32::from(input.after_context % 8),
        max_matches,
        max_files: input.max_files.map(u32::from),
        max_file_size_bytes: input.max_file_size_bytes.map(u64::from),
        max_columns: input.max_columns.map(|c| u32::from(c.max(1))),
        ..Default::default()
    };
    if !input.streamed {
        common::check(common::search(request), max_matches);
        return;
    }
    let Ok(session) = SearchSession::start(request, CancelToken::new(Some(2_000))) else {
        return;
    };
    let batch = u32::from(input.batch.max(1));
    let mut total = 0usize;
    loop {
        match session.next_batch(batch) {
            Ok(b) => {
                b.matches.iter().for_each(common::check_match);
                total += b.matches.len();
                if b.summary.is_some() {
                    break;
                }
            }
            Err(ohmygrep_core::OhMyGrepError::InternalPanic(m)) => panic!("internal panic: {m}"),
            Err(_) => break,
        }
    }
    if let Some(max) = max_matches {
        assert!(total <= max as usize, "{total} streamed over limit {max}");
    }
});
