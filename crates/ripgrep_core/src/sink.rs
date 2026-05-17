use crate::cancel::CancelToken;
use crate::options::{SearchMatch, Submatch};
use crossbeam_channel::Sender;
use grep_matcher::Matcher;
use grep_searcher::{Searcher, Sink, SinkContext, SinkContextKind, SinkError, SinkMatch};
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Cancel check cadence — Sink polls cancel token every N events.
const CANCEL_CHECK_EVERY: usize = 100;
/// Cancel check byte threshold — also poll when accumulated bytes since last check
/// reach this limit. Bounds cancel latency on large/giant lines (e.g. minified JS)
/// where event count alone would not reach CANCEL_CHECK_EVERY in time.
const CANCEL_CHECK_BYTES: usize = 1 << 20; // 1 MiB

/// Sink that pushes matches onto a crossbeam channel. Holds rolling
/// before-context buffers per file and emits after-context lines via
/// SearcherBuilder's context handling.
pub struct ChannelSink<M: Matcher> {
    path: String,
    tx: Sender<SearchMatch>,
    cancel: Arc<CancelToken>,
    match_counter: Arc<AtomicUsize>,
    matcher: M,
    before_buf: VecDeque<String>,
    pending_after: Vec<SearchMatch>,
    events_since_check: usize,
    bytes_since_check: usize,
    before_context: usize,
    after_context: usize,
    /// High-water mark of `pending_after.len()` observed after each push.
    /// Tracked only in test builds to verify the eviction bound.
    #[cfg(test)]
    max_pending_seen: usize,
}

/// Decode a raw line buffer (from `SinkMatch::bytes()` / `SinkContext::bytes()`)
/// to a `String`, stripping the trailing line terminator.
///
/// `grep_searcher` always includes the line terminator in the buffer.  For LF
/// files the buffer ends with `\n`; for CRLF files it ends with `\r\n`.
///
/// Strategy (matches ripgrep's effective CRLF behaviour):
///   1. `trim_end_matches('\n')` — strips the LF (and, for multiline spans,
///      every trailing `\n`; that was the pre-existing behaviour and must not
///      change for multiline matches).
///   2. `.strip_suffix('\r')` — removes **exactly one** trailing `\r` that was
///      immediately before the final `\n`.  Using `strip_suffix` (not
///      `trim_end_matches`) is critical: it removes at most one `\r`, so
///      internal `\r` bytes (including embedded `\r\n` sequences in multiline
///      matches) are untouched.
///
/// Correctness table:
///   `b"foo\n"`      → "foo"      (LF, unchanged)
///   `b"foo\r\n"`    → "foo"      (CRLF, stray \r stripped)
///   `b"foo"`        → "foo"      (no terminator, unchanged)
///   `b"a\rb\n"`     → "a\rb"     (internal bare \r preserved)
///   `b"a\r\nb\r\n"` → "a\r\nb"  (multiline: internal \r\n preserved, trailing stripped)
fn decode_line(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let s = s.trim_end_matches('\n');
    s.strip_suffix('\r').unwrap_or(s).to_string()
}

impl<M: Matcher> ChannelSink<M> {
    pub fn new(
        path: String,
        tx: Sender<SearchMatch>,
        cancel: Arc<CancelToken>,
        match_counter: Arc<AtomicUsize>,
        matcher: M,
        before_context: usize,
        after_context: usize,
    ) -> Self {
        Self {
            path,
            tx,
            cancel,
            match_counter,
            matcher,
            before_buf: VecDeque::with_capacity(before_context.max(1)),
            pending_after: Vec::new(),
            events_since_check: 0,
            bytes_since_check: 0,
            before_context,
            after_context,
            #[cfg(test)]
            max_pending_seen: 0,
        }
    }

    /// Returns the current length of `pending_after`.
    /// Used only in tests to verify the eviction bound.
    #[cfg(test)]
    pub fn pending_len(&self) -> usize {
        self.pending_after.len()
    }

    /// Returns the highest `pending_after.len()` observed after any push during
    /// the search.  Used only in tests to assert the O(after_context) bound.
    #[cfg(test)]
    pub fn max_pending_seen(&self) -> usize {
        self.max_pending_seen
    }

    fn poll_cancel(&mut self, bytes: usize) -> bool {
        self.events_since_check += 1;
        self.bytes_since_check += bytes;
        if self.events_since_check >= CANCEL_CHECK_EVERY
            || self.bytes_since_check >= CANCEL_CHECK_BYTES
        {
            self.events_since_check = 0;
            self.bytes_since_check = 0;
            return self.cancel.is_cancelled();
        }
        false
    }

    fn flush_pending(&mut self) {
        for m in self.pending_after.drain(..) {
            let _ = self.tx.send(m);
            self.match_counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Drain and send the matured prefix of `pending_after`.
    ///
    /// A match `m` is matured once `cur > m.line_number + after_context`: it can
    /// receive no more After lines.  Because matches are pushed in ascending
    /// `line_number` order and `after_context` is constant, matured matches always
    /// form a **prefix** of `pending_after`, so we can drain from the front.
    ///
    /// Call sites:
    ///  • `matched()` with `cur = new_match.line_number` (before pushing new match)
    ///  • `context()` After branch with `cur = abs_line` (before attributing the
    ///    current line — matured matches have `m.line+after < abs_line` and thus
    ///    `abs_line` is outside their window, so calling before attribution is correct
    ///    and simplest)
    fn flush_matured(&mut self, cur: u64) {
        let after = self.after_context as u64;
        // Count the matured prefix (m.line_number + after < cur) without any
        // extra allocation.  The Vec is in ascending line order, so we can stop
        // at the first non-matured entry.
        let n = self
            .pending_after
            .iter()
            .take_while(|m| m.line_number.saturating_add(after) < cur)
            .count();
        for m in self.pending_after.drain(..n) {
            let _ = self.tx.send(m);
            self.match_counter.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[derive(Debug)]
pub struct SinkAbort;

impl std::fmt::Display for SinkAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sink aborted (cancelled)")
    }
}

impl std::error::Error for SinkAbort {}

impl SinkError for SinkAbort {
    fn error_message<T: std::fmt::Display>(_: T) -> Self {
        SinkAbort
    }
    fn error_io(_: io::Error) -> Self {
        SinkAbort
    }
}

impl<M: Matcher> Sink for ChannelSink<M> {
    type Error = SinkAbort;

    fn matched(&mut self, _: &Searcher, m: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        if self.poll_cancel(m.bytes().len()) {
            // Cancel: return Ok(false) — grep_searcher stops this file's search
            // IMMEDIATELY and STILL calls finish(), so flush_pending() runs and
            // already-collected matches are preserved. Do NOT change to Err(SinkAbort):
            // it stops just as immediately but SKIPS finish() (losing pending matches)
            // for zero latency gain. Cancel-poll cadence (event OR byte threshold,
            // v0.2-P6) is the actual latency lever.
            return Ok(false);
        }

        // Decode line; UTF-8 lossy, strip trailing CRLF/LF terminator.
        let line_bytes = m.bytes();
        let line = decode_line(line_bytes);

        // Find submatches inside the line using the matcher.
        let mut submatches = Vec::new();
        let mut cur = 0;
        while let Ok(Some(mat)) = self.matcher.find_at(line_bytes, cur) {
            submatches.push(Submatch {
                start: mat.start() as u32,
                end: mat.end() as u32,
            });
            // Guard against zero-width matches looping forever.
            cur = mat.end().max(mat.start() + 1);
            if cur >= line_bytes.len() {
                break;
            }
        }

        let line_number = m.line_number().unwrap_or_else(|| {
            debug_assert!(false, "ChannelSink: m.line_number() is None; SearcherBuilder must enable line_number(true)");
            0
        });
        let before: Vec<String> = self.before_buf.iter().cloned().collect();

        // Evict any pending match that can no longer receive After lines:
        // m.line + after_context < line_number  ⟹  matured.
        self.flush_matured(line_number);

        let sm = SearchMatch {
            // ACCEPTED per-match owned-String alloc (V3-perf-1, maintainer
            // decision): SearchMatch is a UniFFI Record, so `path` MUST be an
            // owned String per result by FFI contract — Arc<str> on the sink
            // can't remove this. Eliminating it needs an FFI result-model
            // redesign (group-by-file), a breaking public-API change not
            // justified for an IO/regex-bound, max_matches-truncated workload.
            // Do NOT re-raise without that redesign decision.
            path: self.path.clone(),
            line_number,
            line,
            before_context: before,
            after_context: Vec::new(),
            submatches,
        };
        // Defer actual send until after-context is collected.
        self.pending_after.push(sm);
        #[cfg(test)]
        {
            if self.pending_after.len() > self.max_pending_seen {
                self.max_pending_seen = self.pending_after.len();
            }
        }
        Ok(true)
    }

    fn context(&mut self, _: &Searcher, ctx: &SinkContext<'_>) -> Result<bool, Self::Error> {
        if self.poll_cancel(ctx.bytes().len()) {
            // Same contract as matched(): Ok(false) stops immediately, finish()
            // still runs → flush_pending() preserves collected matches. See comment
            // in matched() above for full rationale. Do NOT change to Err(SinkAbort).
            return Ok(false);
        }
        let line = decode_line(ctx.bytes());
        match ctx.kind() {
            SinkContextKind::Before => {
                if self.before_context > 0 {
                    if self.before_buf.len() == self.before_context {
                        self.before_buf.pop_front();
                    }
                    self.before_buf.push_back(line);
                }
            }
            SinkContextKind::After => {
                // Attribute this after-context line to every pending match
                // whose -A window covers it. A match at line `m_line` with
                // an after-context of `N` should receive lines
                // [m_line+1 .. m_line+N] (inclusive).
                // If grep_searcher does not supply an absolute line number,
                // fall back to the last pending match to avoid data loss.
                if let Some(abs_line) = ctx.line_number() {
                    // Evict matured matches before attributing: a matured match
                    // has m.line + after_context < abs_line, so abs_line is outside
                    // its window and it will never receive another After line.
                    self.flush_matured(abs_line);
                    for m in &mut self.pending_after {
                        let m_line = m.line_number;
                        if m_line < abs_line
                            && abs_line <= m_line.saturating_add(self.after_context as u64)
                        {
                            m.after_context.push(line.clone());
                        }
                    }
                } else {
                    debug_assert!(false, "ChannelSink: ctx.line_number() is None; SearcherBuilder must enable line_number(true)");
                    if let Some(m) = self.pending_after.last_mut() {
                        m.after_context.push(line);
                    }
                }
            }
            SinkContextKind::Other => {}
        }
        Ok(true)
    }

    fn context_break(&mut self, _: &Searcher) -> Result<bool, Self::Error> {
        self.flush_pending();
        self.before_buf.clear();
        Ok(true)
    }

    fn finish(&mut self, _: &Searcher, _: &grep_searcher::SinkFinish) -> Result<(), Self::Error> {
        self.flush_pending();
        Ok(())
    }
}
