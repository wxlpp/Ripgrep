use super::cancel::CancelToken;
use super::options::{SearchMatch, SearchRequest};
use super::shared::{Limits, Shared};
use super::sink::{ChannelSink, SinkConfig};
use crossbeam_channel::Sender;
use grep_matcher::Matcher;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Unique temp directory removed on drop; lets tests build fixtures outside any git checkout.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "ohmygrep_test_{tag}_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }

    pub fn path(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }

    pub fn write(&self, name: &str, content: &[u8]) -> String {
        let file = self.0.join(name);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        std::fs::write(&file, content).expect("write fixture");
        file.to_string_lossy().into_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn req(pattern: &str, path: &str) -> SearchRequest {
    SearchRequest {
        pattern: pattern.into(),
        paths: vec![path.into()],
        include_hidden: true,
        ..Default::default()
    }
}

/// A sink over a fresh shared state without limits, for driving grep-searcher directly.
pub fn test_sink<M: Matcher>(
    path: &str,
    tx: Sender<SearchMatch>,
    cancel: Arc<CancelToken>,
    matcher: M,
) -> ChannelSink<M> {
    let shared = Arc::new(Shared::new(cancel, None, Limits::default()));
    let config = SinkConfig {
        before_context: 0,
        after_context: 0,
        max_columns: None,
        explicit: false,
    };
    ChannelSink::new(path.to_string(), tx, shared, matcher, config)
}
