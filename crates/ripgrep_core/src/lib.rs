//! ripgrep_core — Swift-facing wrapper around ripgrep's reusable crates.
//!
//! Public surface is generated via UniFFI; see `lib.rs` `uniffi::setup_scaffolding!()`.

uniffi::setup_scaffolding!();

mod error;
mod options;

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/error_tests.rs"]
mod error_tests;

#[cfg(test)]
#[path = "tests/options_tests.rs"]
mod options_tests;

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
#[path = "tests/search_matcher_tests.rs"]
mod search_matcher_tests;

#[cfg(test)]
#[path = "tests/sink_tests.rs"]
mod sink_tests;

#[cfg(test)]
#[path = "tests/cancel_tests.rs"]
mod cancel_tests;

#[cfg(test)]
#[path = "tests/walker_tests.rs"]
mod walker_tests;

#[cfg(test)]
#[path = "tests/search_e2e_tests.rs"]
mod search_e2e_tests;

#[cfg(test)]
#[path = "tests/ffi_export_smoke.rs"]
mod ffi_export_smoke;

#[cfg(test)]
#[path = "tests/context_tests.rs"]
mod context_tests;

#[cfg(test)]
#[path = "tests/limits_tests.rs"]
mod limits_tests;

#[cfg(test)]
#[path = "tests/adjacent_after_context_tests.rs"]
mod adjacent_after_context_tests;

#[cfg(test)]
#[path = "tests/panic_tests.rs"]
mod panic_tests;

/// Regression test: cancel returns Ok(false) → finish() still runs → flush_pending()
/// preserves already-collected matches. DO NOT change the cancel path to Err(SinkAbort)
/// without first understanding why this test exists (see sink.rs matched() comment).
#[cfg(test)]
#[path = "tests/cancel_preserves_pending_matches_tests.rs"]
mod cancel_preserves_pending_matches_tests;

/// TDD for v0.2-P6a: verify the hybrid event-OR-byte cancel cadence fires on
/// giant lines that would NOT reach CANCEL_CHECK_EVERY (100) events.
///
/// Non-vacuity: with the OLD event-only cadence (ignoring the bytes param), a
/// 3-line input produces only 3 events — far below 100 — so the cancel token is
/// never polled and all lines are processed regardless of cancellation. The new
/// byte-cadence threshold (1 MiB per event) fires on the very first line and
/// stops the search early.
#[cfg(test)]
#[path = "tests/cancel_byte_cadence_tests.rs"]
mod cancel_byte_cadence_tests;

#[cfg(test)]
#[path = "tests/external_cancel_tests.rs"]
mod external_cancel_tests;

/// TDD for v0.3-perf-2: bound pending_after by evicting matured matches.
/// Attribution must be identical with/without eviction; the bound (pending_len ≤
/// after_context+1) is the perf property.
#[cfg(test)]
#[path = "tests/pending_after_bound_tests.rs"]
mod pending_after_bound_tests;

/// TDD for v0.4-CRLF-1: CRLF line terminator must not leave a stray `\r` in
/// emitted match/context lines. Reverting the fix in sink.rs makes the
/// assertions fail with trailing `\r` on every CRLF-file line.
#[cfg(test)]
#[path = "tests/crlf_tests.rs"]
mod crlf_tests;
