use crate::options::SearchWarning;
use std::path::Path;
use std::sync::Mutex;

pub const MAX_WARNINGS: usize = 100;

/// Thread-safe warning collector shared by walker workers; keeps the first
/// `MAX_WARNINGS` and counts the rest.
#[derive(Default)]
pub struct Warnings {
    inner: Mutex<(Vec<SearchWarning>, usize)>,
}

impl Warnings {
    pub fn push(&self, path: impl Into<String>, message: impl Into<String>) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if guard.0.len() < MAX_WARNINGS {
            guard.0.push(SearchWarning {
                path: path.into(),
                message: message.into(),
            });
        } else {
            guard.1 += 1;
        }
    }

    pub fn push_walk_error(&self, err: &ignore::Error) {
        let (path, message) = split_walk_error(err);
        self.push(path, message);
    }

    pub fn take(&self) -> Vec<SearchWarning> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let (mut list, dropped) = std::mem::take(&mut *guard);
        if dropped > 0 {
            list.push(SearchWarning {
                path: String::new(),
                message: format!("{dropped} more warnings omitted"),
            });
        }
        list
    }
}

/// Peels path/depth/line wrappers so the path is reported once, not repeated in the message.
fn split_walk_error(err: &ignore::Error) -> (String, String) {
    match err {
        ignore::Error::WithPath { path, err } => {
            let (_, message) = split_walk_error(err);
            (display(path), message)
        }
        ignore::Error::WithDepth { err, .. } => split_walk_error(err),
        ignore::Error::Loop { child, .. } => (display(child), err.to_string()),
        _ => (String::new(), err.to_string()),
    }
}

fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
