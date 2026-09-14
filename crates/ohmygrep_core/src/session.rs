use crate::cancel::CancelToken;
use crate::error::OhMyGrepError;
use crate::options::{SearchBatch, SearchMatch, SearchRequest, SearchSummary};
use crate::search::{panic_safe, prepare};
use crate::shared::Limits;
use crossbeam_channel::{bounded, Receiver, TryRecvError};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

/// Matches buffered between the walk and the consumer; the walk blocks beyond this.
const CHANNEL_CAPACITY: usize = 256;

type Worker = JoinHandle<Result<SearchSummary, OhMyGrepError>>;

/// A search running on a background thread whose matches are pulled in batches.
#[derive(uniffi::Object)]
pub struct SearchSession {
    rx: Receiver<SearchMatch>,
    cancel: Arc<CancelToken>,
    worker: Mutex<Option<Worker>>,
    finished: AtomicBool,
}

#[uniffi::export]
impl SearchSession {
    /// Validates the request synchronously, then starts the walk.
    #[uniffi::constructor]
    pub fn start(
        request: SearchRequest,
        cancel: Arc<CancelToken>,
    ) -> Result<Arc<Self>, OhMyGrepError> {
        Self::start_with_limits(request, cancel, Limits::default())
    }

    /// Blocks until at least one match is available or the search ends, then returns
    /// up to `max` matches. The final batch carries the summary; after a cancel it
    /// arrives without draining buffered matches.
    pub fn next_batch(&self, max: u32) -> Result<SearchBatch, OhMyGrepError> {
        panic_safe(AssertUnwindSafe(|| self.next_batch_inner(max as usize)))?
    }
}

impl SearchSession {
    pub(crate) fn start_with_limits(
        request: SearchRequest,
        cancel: Arc<CancelToken>,
        limits: Limits,
    ) -> Result<Arc<Self>, OhMyGrepError> {
        panic_safe(AssertUnwindSafe(|| {
            let prepared = prepare(request, limits)?;
            let (tx, rx) = bounded(CHANNEL_CAPACITY);
            let token = Arc::clone(&cancel);
            let worker = std::thread::Builder::new()
                .name("oh-my-grep-search".into())
                .spawn(move || {
                    panic_safe(AssertUnwindSafe(move || prepared.run(token, tx)))
                        .and_then(|result| result)
                })
                .map_err(|e| OhMyGrepError::Io(format!("cannot start search thread: {e}")))?;
            Ok(Arc::new(SearchSession {
                rx,
                cancel,
                worker: Mutex::new(Some(worker)),
                finished: AtomicBool::new(false),
            }))
        }))?
    }

    fn next_batch_inner(&self, max: usize) -> Result<SearchBatch, OhMyGrepError> {
        if max == 0 {
            return Err(OhMyGrepError::InvalidArguments(
                "batch size must be at least 1".into(),
            ));
        }
        if self.finished.load(Ordering::SeqCst) {
            return Err(OhMyGrepError::InvalidArguments("session finished".into()));
        }
        if self.cancel.is_cancelled() {
            // Buffered matches are dropped, so the result is incomplete even if the walk had ended.
            return self.finish(Vec::new(), true);
        }
        let mut matches = Vec::new();
        match self.rx.recv() {
            Ok(m) => matches.push(m),
            Err(_) => return self.finish(matches, false),
        }
        while matches.len() < max {
            match self.rx.try_recv() {
                Ok(m) => matches.push(m),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return self.finish(matches, false),
            }
        }
        Ok(SearchBatch {
            matches,
            summary: None,
        })
    }

    /// Joins the walk. Only called once the channel is disconnected or the token is
    /// cancelled, so the worker is finished or exits at its next stop check.
    fn finish(
        &self,
        matches: Vec<SearchMatch>,
        dropped_buffered: bool,
    ) -> Result<SearchBatch, OhMyGrepError> {
        self.finished.store(true, Ordering::SeqCst);
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .ok_or_else(|| OhMyGrepError::InvalidArguments("session finished".into()))?;
        let mut summary = worker
            .join()
            .map_err(|_| OhMyGrepError::InternalPanic("search thread panicked".into()))??;
        summary.cancelled |= dropped_buffered && !self.rx.is_empty();
        Ok(SearchBatch {
            matches,
            summary: Some(summary),
        })
    }
}

#[cfg(test)]
impl SearchSession {
    pub fn worker_finished(&self) -> bool {
        self.worker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .is_none_or(|w| w.is_finished())
    }

    pub fn buffered(&self) -> usize {
        self.rx.len()
    }
}

impl Drop for SearchSession {
    fn drop(&mut self) {
        // Never join here: the last reference may be released on a Swift
        // cooperative thread. The worker exits at its next stop check.
        // This cancels the token given to `start`, so tokens must not be shared.
        self.cancel.cancel();
    }
}
