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
    before_context: usize,
    after_context: usize,
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
            before_context,
            after_context,
        }
    }

    fn poll_cancel(&mut self) -> bool {
        self.events_since_check += 1;
        if self.events_since_check >= CANCEL_CHECK_EVERY {
            self.events_since_check = 0;
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
        if self.poll_cancel() {
            // Cancel: return Ok(false) — grep_searcher stops this file's search
            // IMMEDIATELY and STILL calls finish(), so flush_pending() runs and
            // already-collected matches are preserved. Do NOT change to Err(SinkAbort):
            // it stops just as immediately but SKIPS finish() (losing pending matches)
            // for zero latency gain. Cancel-poll cadence (CANCEL_CHECK_EVERY) is the
            // actual latency lever (see v0.2-P6).
            return Ok(false);
        }

        // Decode line; UTF-8 lossy.
        let line_bytes = m.bytes();
        let line = String::from_utf8_lossy(line_bytes)
            .trim_end_matches('\n')
            .to_string();

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

        let line_number = m.line_number().unwrap_or(0); // line numbers are enabled (search.rs sb.line_number(true)); 0 only on searcher misconfig
        let before: Vec<String> = self.before_buf.iter().cloned().collect();

        let sm = SearchMatch {
            path: self.path.clone(),
            line_number,
            line,
            before_context: before,
            after_context: Vec::new(),
            submatches,
        };
        // Defer actual send until after-context is collected.
        self.pending_after.push(sm);
        Ok(true)
    }

    fn context(&mut self, _: &Searcher, ctx: &SinkContext<'_>) -> Result<bool, Self::Error> {
        if self.poll_cancel() {
            // Same contract as matched(): Ok(false) stops immediately, finish()
            // still runs → flush_pending() preserves collected matches. See comment
            // in matched() above for full rationale. Do NOT change to Err(SinkAbort).
            return Ok(false);
        }
        let line = String::from_utf8_lossy(ctx.bytes())
            .trim_end_matches('\n')
            .to_string();
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
