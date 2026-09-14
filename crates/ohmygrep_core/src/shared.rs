use crate::cancel::CancelToken;
use crate::warnings::Warnings;
use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

const MIB: usize = 1 << 20;

/// Memory limits for one search.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Per-thread cap on the line buffer (single-line mode).
    pub line_heap: usize,
    /// Cap shared by all threads for whole-file buffers (multiline mode).
    pub multiline_budget: usize,
    pub threads: usize,
}

impl Default for Limits {
    fn default() -> Self {
        let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
        Limits {
            line_heap: 16 * MIB,
            multiline_budget: 128 * MIB,
            threads: cores.min(8),
        }
    }
}

/// State shared by the walker workers of one search.
pub struct Shared {
    pub cancel: Arc<CancelToken>,
    /// Stop everything and drop undelivered matches (cancel or panic).
    abort: AtomicBool,
    /// The match limit was reached: stop searching, but deliver reserved matches.
    limit_reached: AtomicBool,
    counter: AtomicUsize,
    max_matches: Option<usize>,
    pub files: AtomicUsize,
    pub warnings: Warnings,
    panic: Mutex<Option<String>>,
    multiline_available: AtomicUsize,
    pub limits: Limits,
}

impl Shared {
    pub fn new(cancel: Arc<CancelToken>, max_matches: Option<usize>, limits: Limits) -> Self {
        let shared = Shared {
            cancel,
            abort: AtomicBool::new(false),
            limit_reached: AtomicBool::new(false),
            counter: AtomicUsize::new(0),
            max_matches,
            files: AtomicUsize::new(0),
            warnings: Warnings::default(),
            panic: Mutex::new(None),
            multiline_available: AtomicUsize::new(limits.multiline_budget),
            limits,
        };
        if max_matches == Some(0) {
            shared.limit_reached.store(true, Ordering::Relaxed);
        }
        shared
    }

    pub fn aborted(&self) -> bool {
        self.abort.load(Ordering::Relaxed) || self.cancel.is_cancelled()
    }

    pub fn limit_reached(&self) -> bool {
        self.limit_reached.load(Ordering::Relaxed)
    }

    pub fn should_stop(&self) -> bool {
        self.aborted() || self.limit_reached()
    }

    /// Takes one match slot. The reservation that fills the limit also flags it,
    /// so other workers stop before looking for a match that cannot be returned.
    pub fn try_reserve(&self) -> bool {
        let Some(max) = self.max_matches else {
            self.counter.fetch_add(1, Ordering::Relaxed);
            return true;
        };
        let reserved = self
            .counter
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |c| {
                (c < max).then_some(c + 1)
            });
        match reserved {
            Ok(previous) => {
                if previous + 1 >= max {
                    self.limit_reached.store(true, Ordering::Relaxed);
                }
                true
            }
            Err(_) => false,
        }
    }

    pub fn truncated(&self) -> bool {
        self.max_matches
            .is_some_and(|max| self.counter.load(Ordering::SeqCst) >= max)
    }

    pub fn record_panic(&self, message: String) {
        let mut slot = self.panic.lock().unwrap_or_else(PoisonError::into_inner);
        slot.get_or_insert(message);
        self.abort.store(true, Ordering::Relaxed);
    }

    pub fn take_panic(&self) -> Option<String> {
        self.panic
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// Reserves `bytes` of the multiline budget, waiting while other files hold it.
    /// Returns `None` if the request can never fit or the search stops first.
    pub fn reserve_multiline(self: &Arc<Self>, bytes: usize) -> Option<BudgetGuard> {
        if bytes > self.limits.multiline_budget {
            return None;
        }
        loop {
            let taken =
                self.multiline_available
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |free| {
                        free.checked_sub(bytes)
                    });
            if taken.is_ok() {
                return Some(BudgetGuard {
                    shared: Arc::clone(self),
                    bytes,
                });
            }
            if self.should_stop() {
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

pub struct BudgetGuard {
    shared: Arc<Shared>,
    bytes: usize,
}

impl Drop for BudgetGuard {
    fn drop(&mut self) {
        self.shared
            .multiline_available
            .fetch_add(self.bytes, Ordering::SeqCst);
    }
}

/// Reports EOF once the search should stop, checked every 1 MiB, so a large file
/// without matches does not delay cancellation or the match limit.
pub struct StoppableReader<R> {
    inner: R,
    shared: Arc<Shared>,
    since_check: usize,
    /// Set while a match in this file still collects after-context; the match limit
    /// then does not stop reading, or that context would end in a cut-off line.
    collecting: Option<Arc<AtomicBool>>,
    /// Once stopped, stay at EOF: grep-searcher reads again after an EOF that
    /// leaves a partial line in its buffer.
    stopped: bool,
}

impl<R> StoppableReader<R> {
    pub fn new(inner: R, shared: Arc<Shared>, collecting: Option<Arc<AtomicBool>>) -> Self {
        StoppableReader {
            inner,
            shared,
            since_check: 0,
            collecting,
            stopped: false,
        }
    }

    fn should_stop(&self) -> bool {
        let collecting = self
            .collecting
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Relaxed));
        self.shared.aborted() || (self.shared.limit_reached() && !collecting)
    }
}

impl<R: Read> Read for StoppableReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.stopped && self.since_check >= MIB {
            self.since_check = 0;
            self.stopped = self.should_stop();
        }
        if self.stopped {
            // EOF rather than an error: `Interrupted` would be retried forever.
            return Ok(0);
        }
        let n = self.inner.read(buf)?;
        self.since_check += n;
        Ok(n)
    }
}
