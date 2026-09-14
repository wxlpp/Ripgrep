//! ohmygrep_core — Swift-facing wrapper around ripgrep's reusable crates, exported via UniFFI.

uniffi::setup_scaffolding!();

mod cancel;
mod columns;
mod error;
mod options;
mod search;
mod session;
mod shared;
mod sink;
mod warnings;

use cancel::CancelToken;
use error::OhMyGrepError;
use options::{SearchRequest, SearchResult};
use std::sync::Arc;

#[uniffi::export]
pub fn search_blocking(
    request: SearchRequest,
    cancel: Arc<CancelToken>,
) -> Result<SearchResult, OhMyGrepError> {
    search::search_blocking(request, cancel)
}

#[cfg(test)]
#[path = "tests/support.rs"]
mod support;

#[cfg(test)]
#[path = "tests/cancel_byte_cadence_tests.rs"]
mod cancel_byte_cadence_tests;
#[cfg(test)]
#[path = "tests/cancel_preserves_pending_matches_tests.rs"]
mod cancel_preserves_pending_matches_tests;
#[cfg(test)]
#[path = "tests/cancel_tests.rs"]
mod cancel_tests;
#[cfg(test)]
#[path = "tests/columns_tests.rs"]
mod columns_tests;
#[cfg(test)]
#[path = "tests/context_tests.rs"]
mod context_tests;
#[cfg(test)]
#[path = "tests/error_tests.rs"]
mod error_tests;
#[cfg(test)]
#[path = "tests/external_cancel_tests.rs"]
mod external_cancel_tests;
#[cfg(test)]
#[path = "tests/ffi_export_smoke.rs"]
mod ffi_export_smoke;
#[cfg(test)]
#[path = "tests/limits_and_session_tests.rs"]
mod limits_and_session_tests;
#[cfg(test)]
#[path = "tests/limits_tests.rs"]
mod limits_tests;
#[cfg(test)]
#[path = "tests/line_decoding_tests.rs"]
mod line_decoding_tests;
#[cfg(test)]
#[path = "tests/options_tests.rs"]
mod options_tests;
#[cfg(test)]
#[path = "tests/panic_tests.rs"]
mod panic_tests;
#[cfg(test)]
#[path = "tests/paths_and_warnings_tests.rs"]
mod paths_and_warnings_tests;
#[cfg(test)]
#[path = "tests/search_e2e_tests.rs"]
mod search_e2e_tests;
#[cfg(test)]
#[path = "tests/search_matcher_tests.rs"]
mod search_matcher_tests;
#[cfg(test)]
#[path = "tests/sink_tests.rs"]
mod sink_tests;
#[cfg(test)]
#[path = "tests/walker_tests.rs"]
mod walker_tests;
