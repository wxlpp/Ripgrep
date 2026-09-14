use crate::cancel::CancelToken;
use crate::options::{SearchMatch, Submatch};
use crossbeam_channel::Sender;
use grep_matcher::Matcher;
use grep_searcher::{Searcher, Sink, SinkContext, SinkContextKind, SinkError, SinkMatch};
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Poll the cancel token every N sink events...
const CANCEL_CHECK_EVERY: usize = 100;
/// ...or after this many bytes, so a few giant lines cannot delay cancellation.
const CANCEL_CHECK_BYTES: usize = 1 << 20;

/// Per-file sink. A match owns only the contiguous non-match lines next to it:
/// before-context never reaches back past the previous match and after-context
/// ends at the next match. A multiline match's `line` keeps its inner `\n`s.
pub struct ChannelSink<M: Matcher> {
    path: String,
    tx: Sender<SearchMatch>,
    cancel: Arc<CancelToken>,
    match_counter: Arc<AtomicUsize>,
    matcher: M,
    before_context: usize,
    before_buf: VecDeque<String>,
    pending: Option<SearchMatch>,
    events_since_check: usize,
    bytes_since_check: usize,
}

/// The bytes without one trailing line terminator (`\n` or `\r\n`).
fn content(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    bytes.strip_suffix(b"\r").unwrap_or(bytes)
}

fn decode_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(content(bytes)).into_owned()
}

impl<M: Matcher> ChannelSink<M> {
    pub fn new(
        path: String,
        tx: Sender<SearchMatch>,
        cancel: Arc<CancelToken>,
        match_counter: Arc<AtomicUsize>,
        matcher: M,
        before_context: usize,
    ) -> Self {
        Self {
            path,
            tx,
            cancel,
            match_counter,
            matcher,
            before_context,
            before_buf: VecDeque::with_capacity(before_context),
            pending: None,
            events_since_check: 0,
            bytes_since_check: 0,
        }
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
        if let Some(m) = self.pending.take() {
            let _ = self.tx.send(m);
            self.match_counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Like rg: multiline matches are searched with their terminators, single
    /// lines without the trailing `\n`. Offsets are clamped to the decoded `line`.
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
}

#[derive(Debug)]
pub struct SinkAbort;

impl std::fmt::Display for SinkAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sink aborted")
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

    fn matched(&mut self, searcher: &Searcher, m: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        // Cancel with Ok(false), not Err: grep-searcher still calls finish(),
        // which flushes the pending match instead of dropping it.
        if self.poll_cancel(m.bytes().len()) {
            return Ok(false);
        }
        self.flush_pending();
        let line_number = m.line_number().unwrap_or_else(|| {
            debug_assert!(false, "SearcherBuilder must enable line_number(true)");
            0
        });
        self.pending = Some(SearchMatch {
            // Owned per match: SearchMatch is a UniFFI record.
            path: self.path.clone(),
            line_number,
            line: decode_line(m.bytes()),
            before_context: self.before_buf.drain(..).collect(),
            after_context: Vec::new(),
            submatches: self.submatches(m.bytes(), searcher.multi_line_with_matcher(&self.matcher)),
        });
        Ok(true)
    }

    fn context(&mut self, _: &Searcher, ctx: &SinkContext<'_>) -> Result<bool, Self::Error> {
        if self.poll_cancel(ctx.bytes().len()) {
            return Ok(false);
        }
        match ctx.kind() {
            SinkContextKind::Before => {
                if self.before_context > 0 {
                    if self.before_buf.len() == self.before_context {
                        self.before_buf.pop_front();
                    }
                    self.before_buf.push_back(decode_line(ctx.bytes()));
                }
            }
            SinkContextKind::After => {
                if let Some(p) = self.pending.as_mut() {
                    p.after_context.push(decode_line(ctx.bytes()));
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
