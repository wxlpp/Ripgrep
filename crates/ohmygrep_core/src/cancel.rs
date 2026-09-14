use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cancellation token for in-flight searches. Construction takes an
/// optional timeout (deadline = now + timeout). `is_cancelled` returns
/// true if either an explicit `cancel()` was called or the deadline
/// elapsed. The deadline is immutable after construction.
#[derive(Debug, uniffi::Object)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

#[uniffi::export]
impl CancelToken {
    #[uniffi::constructor]
    pub fn new(timeout_ms: Option<u64>) -> Arc<Self> {
        Arc::new(Self {
            flag: Arc::new(AtomicBool::new(false)),
            deadline: timeout_ms.map(|ms| Instant::now() + Duration::from_millis(ms)),
        })
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        if self.flag.load(Ordering::Relaxed) {
            return true;
        }
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }
}
