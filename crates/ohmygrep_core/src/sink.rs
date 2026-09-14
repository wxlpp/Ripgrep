use crate::columns::{cut_match, cut_prefix};
use crate::options::{SearchMatch, Submatch};
use crate::shared::Shared;
use crossbeam_channel::{SendTimeoutError, Sender};
use grep_matcher::Matcher;
use grep_searcher::{
    Searcher, Sink, SinkContext, SinkContextKind, SinkError, SinkFinish, SinkMatch,
};
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::Duration;

/// Poll for stop every N sink events...
const STOP_CHECK_EVERY: usize = 100;
/// ...or after this many bytes, so a few giant lines cannot delay it.
const STOP_CHECK_BYTES: usize = 1 << 20;
/// How long a blocked send waits before re-checking cancellation.
const SEND_POLL: Duration = Duration::from_millis(50);

#[cfg(test)]
pub const TEST_PANIC_MARKER: &str = "__ohmygrep_test_panic__";

/// Per-file search settings for the sink.
#[derive(Clone, Copy)]
pub struct SinkConfig {
    pub before_context: usize,
    pub after_context: usize,
    pub max_columns: Option<usize>,
    /// The caller named this path directly (affects binary reporting).
    pub explicit: bool,
}

/// Per-file sink. A match owns only the contiguous non-match lines next to it:
/// before-context never reaches back past the previous match and after-context
/// ends at the next match. A multiline match's `line` keeps its inner `\n`s.
pub struct ChannelSink<M: Matcher> {
    path: String,
    tx: Sender<SearchMatch>,
    shared: Arc<Shared>,
    matcher: M,
    config: SinkConfig,
    before_buf: VecDeque<String>,
    pending: Option<SearchMatch>,
    events_since_check: usize,
    bytes_since_check: usize,
    binary_offset: Option<u64>,
    matches_in_file: u64,
}

/// The bytes without one trailing line terminator (`\n` or `\r\n`).
fn content(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    bytes.strip_suffix(b"\r").unwrap_or(bytes)
}

impl<M: Matcher> ChannelSink<M> {
    pub fn new(
        path: String,
        tx: Sender<SearchMatch>,
        shared: Arc<Shared>,
        matcher: M,
        config: SinkConfig,
    ) -> Self {
        Self {
            path,
            tx,
            shared,
            matcher,
            config,
            before_buf: VecDeque::with_capacity(config.before_context),
            pending: None,
            events_since_check: 0,
            bytes_since_check: 0,
            binary_offset: None,
            matches_in_file: 0,
        }
    }

    fn poll_stop(&mut self, bytes: usize) -> bool {
        self.events_since_check += 1;
        self.bytes_since_check += bytes;
        if self.events_since_check >= STOP_CHECK_EVERY || self.bytes_since_check >= STOP_CHECK_BYTES
        {
            self.events_since_check = 0;
            self.bytes_since_check = 0;
            return self.shared.should_stop();
        }
        false
    }

    /// Sends with back-pressure. Gives up only when the search is aborted or the
    /// receiver is gone, never merely because the match limit was reached.
    fn send(&mut self, mut m: SearchMatch) -> bool {
        loop {
            match self.tx.send_timeout(m, SEND_POLL) {
                Ok(()) => return true,
                Err(SendTimeoutError::Timeout(back)) => {
                    if self.shared.aborted() {
                        return false;
                    }
                    m = back;
                }
                Err(SendTimeoutError::Disconnected(_)) => return false,
            }
        }
    }

    fn flush_pending(&mut self) -> bool {
        match self.pending.take() {
            Some(m) => self.send(m),
            None => true,
        }
    }

    /// Like rg: multiline matches are searched with their terminators, single
    /// lines without the trailing `\n`. Offsets are clamped to the content.
    fn submatches(&self, bytes: &[u8], multi_line: bool) -> Vec<Submatch> {
        let haystack = if multi_line {
            bytes
        } else {
            bytes.strip_suffix(b"\n").unwrap_or(bytes)
        };
        let limit = content(bytes).len();
        let mut out = Vec::new();
        let mut at = 0;
        while let Ok(Some(mat)) = self.matcher.find_at(haystack, at) {
            out.push(Submatch {
                start: mat.start().min(limit) as u32,
                end: mat.end().min(limit) as u32,
            });
            at = mat.end().max(mat.start() + 1);
            if at >= haystack.len() {
                break;
            }
        }
        out
    }

    fn binary_warning(&self) {
        if let (Some(offset), true) = (self.binary_offset, self.matches_in_file > 0) {
            let message = if self.config.explicit {
                format!("binary file matches (found \"\\0\" byte around offset {offset})")
            } else {
                format!("stopped searching binary file after match (found \"\\0\" byte around offset {offset})")
            };
            self.shared.warnings.push(self.path.clone(), message);
        }
    }

    /// The searcher skips `finish` when it fails, so deliver what was found and report why it stopped.
    pub fn finish_after_error(&mut self, message: &str) {
        self.flush_pending();
        self.binary_warning();
        let reason = if message.starts_with("configured allocation limit") {
            format!(
                "stopped: a line needs more than {} MiB of memory",
                self.shared.limits.line_heap >> 20
            )
        } else {
            format!("stopped: {message}")
        };
        let note = if self.matches_in_file > 0 {
            "; earlier matches in this file were reported"
        } else {
            ""
        };
        self.shared
            .warnings
            .push(self.path.clone(), format!("{reason}{note}"));
    }
}

/// A searcher failure for one file (e.g. unreadable), surfaced as a warning.
#[derive(Debug)]
pub struct SearchFailure(String);

impl std::fmt::Display for SearchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SearchFailure {}

impl SinkError for SearchFailure {
    fn error_message<T: std::fmt::Display>(message: T) -> Self {
        SearchFailure(message.to_string())
    }
    fn error_io(err: io::Error) -> Self {
        SearchFailure(err.to_string())
    }
}

impl<M: Matcher> Sink for ChannelSink<M> {
    type Error = SearchFailure;

    fn matched(&mut self, searcher: &Searcher, m: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        #[cfg(test)]
        if self.path.contains(TEST_PANIC_MARKER) {
            panic!("test panic in sink");
        }
        // Stop with Ok(false), not Err: grep-searcher still calls finish(),
        // which delivers the pending match instead of dropping it.
        if self.poll_stop(m.bytes().len()) {
            return Ok(false);
        }
        self.matches_in_file += 1;
        // Like rg, a named binary file reports that it matches instead of returning lines.
        if self.config.explicit && self.binary_offset.is_some() {
            return Ok(false);
        }
        if !self.flush_pending() || !self.shared.try_reserve() {
            return Ok(false);
        }
        let line_number = m.line_number().unwrap_or_else(|| {
            debug_assert!(false, "SearcherBuilder must enable line_number(true)");
            0
        });
        let multi_line = searcher.multi_line_with_matcher(&self.matcher);
        let cut = cut_match(
            content(m.bytes()),
            self.submatches(m.bytes(), multi_line),
            self.config.max_columns,
            multi_line,
        );
        let found = SearchMatch {
            // Owned per match: SearchMatch is a UniFFI record.
            path: self.path.clone(),
            line_number,
            line: cut.line,
            before_context: self.before_buf.drain(..).collect(),
            after_context: Vec::new(),
            submatches: cut.submatches,
            line_offset: cut.offset,
            line_truncated: cut.truncated,
        };
        if self.config.after_context == 0 {
            // Nothing more to attach, so do not hold it until the next match.
            return Ok(self.send(found));
        }
        self.pending = Some(found);
        Ok(true)
    }

    fn context(&mut self, _: &Searcher, ctx: &SinkContext<'_>) -> Result<bool, Self::Error> {
        if self.poll_stop(ctx.bytes().len()) {
            return Ok(false);
        }
        let max_columns = self.config.max_columns;
        match ctx.kind() {
            SinkContextKind::Before => {
                if self.config.before_context > 0 {
                    if self.before_buf.len() == self.config.before_context {
                        self.before_buf.pop_front();
                    }
                    self.before_buf
                        .push_back(cut_prefix(content(ctx.bytes()), max_columns));
                }
            }
            SinkContextKind::After => {
                if let Some(p) = self.pending.as_mut() {
                    p.after_context
                        .push(cut_prefix(content(ctx.bytes()), max_columns));
                }
            }
            SinkContextKind::Other => {}
        }
        Ok(true)
    }

    fn context_break(&mut self, _: &Searcher) -> Result<bool, Self::Error> {
        self.before_buf.clear();
        Ok(self.flush_pending())
    }

    fn binary_data(&mut self, _: &Searcher, offset: u64) -> Result<bool, Self::Error> {
        self.binary_offset.get_or_insert(offset);
        Ok(true)
    }

    fn finish(&mut self, _: &Searcher, finish: &SinkFinish) -> Result<(), Self::Error> {
        self.flush_pending();
        if self.binary_offset.is_none() {
            self.binary_offset = finish.binary_byte_offset();
        }
        self.binary_warning();
        Ok(())
    }
}
