use crate::error::RipgrepError;
use crate::options::SearchRequest;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};

pub fn build_matcher(req: &SearchRequest) -> Result<RegexMatcher, RipgrepError> {
    let mut b = RegexMatcherBuilder::new();
    b.case_insensitive(req.case_insensitive);
    b.case_smart(req.smart_case);
    b.multi_line(req.multiline);
    b.build(&req.pattern)
        .map_err(|e| RipgrepError::InvalidPattern(e.to_string()))
}

use crate::cancel::CancelToken;
use crate::options::{SearchMatch, SearchResult};
use crate::sink::ChannelSink;
use crossbeam_channel::unbounded;
use grep_searcher::SearcherBuilder;
use ignore::WalkState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub fn panic_safe<F, R>(f: F) -> Result<R, RipgrepError>
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(r) => Ok(r),
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&'static str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic payload".to_string()
            };
            Err(RipgrepError::InternalPanic(msg))
        }
    }
}

#[cfg(test)]
pub fn force_panic_for_test() -> Result<(), RipgrepError> {
    panic_safe(|| panic!("intentional test panic"))
}

pub fn search_blocking(
    req: SearchRequest,
    cancel: Arc<CancelToken>,
) -> Result<SearchResult, RipgrepError> {
    panic_safe(move || search_blocking_inner(req, cancel))?
}

fn search_blocking_inner(
    req: SearchRequest,
    cancel: Arc<CancelToken>,
) -> Result<SearchResult, RipgrepError> {
    let start = Instant::now();

    let matcher = build_matcher(&req)?;
    let walker = build_walker(&req)?.build_parallel();

    let (tx, rx) = unbounded::<SearchMatch>();
    let match_counter = Arc::new(AtomicUsize::new(0));
    let file_counter = Arc::new(AtomicUsize::new(0));

    let max_matches = req.max_matches.map(|n| n as usize);
    let max_files = req.max_files.map(|n| n as usize);
    let before = req.before_context as usize;
    let after = req.after_context as usize;
    let multiline = req.multiline;

    walker.run(|| {
        let tx = tx.clone();
        let cancel = Arc::clone(&cancel);
        let match_counter = Arc::clone(&match_counter);
        let file_counter = Arc::clone(&file_counter);
        let matcher = matcher.clone();
        Box::new(move |entry| {
            if cancel.is_cancelled() {
                return WalkState::Quit;
            }
            if let Some(max) = max_matches {
                if match_counter.load(Ordering::Relaxed) >= max {
                    return WalkState::Quit;
                }
            }
            if let Some(max) = max_files {
                if file_counter.load(Ordering::Relaxed) >= max {
                    return WalkState::Quit;
                }
            }
            let entry = match entry {
                Ok(e) => e,
                Err(_) => return WalkState::Continue,
            };
            if !entry.file_type().map_or(false, |t| t.is_file()) {
                return WalkState::Continue;
            }
            file_counter.fetch_add(1, Ordering::Relaxed);

            let path = entry.path().to_string_lossy().to_string();
            let mut sink = ChannelSink::new(
                path,
                tx.clone(),
                Arc::clone(&cancel),
                Arc::clone(&match_counter),
                matcher.clone(),
                before,
            );
            let mut sb = SearcherBuilder::new();
            sb.before_context(before);
            sb.after_context(after);
            sb.multi_line(multiline);
            let _ = sb.build().search_path(&matcher, entry.path(), &mut sink);
            WalkState::Continue
        })
    });

    drop(tx);
    let mut matches: Vec<SearchMatch> = rx.iter().collect();
    matches.sort_by(|a, b| {
        (a.path.as_str(), a.line_number).cmp(&(b.path.as_str(), b.line_number))
    });

    let truncated = if let Some(max) = max_matches {
        if matches.len() > max {
            matches.truncate(max);
            true
        } else {
            false
        }
    } else {
        false
    };

    Ok(SearchResult {
        matches,
        truncated,
        cancelled: cancel.is_cancelled(),
        files_searched: file_counter.load(Ordering::Relaxed) as u64,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use std::path::Path;

pub fn build_walker(req: &SearchRequest) -> Result<WalkBuilder, RipgrepError> {
    if req.paths.is_empty() {
        return Err(RipgrepError::PathNotFound("(empty paths)".into()));
    }
    for p in &req.paths {
        if !Path::new(p).exists() {
            return Err(RipgrepError::PathNotFound(p.clone()));
        }
    }

    let first = &req.paths[0];
    let mut wb = WalkBuilder::new(first);
    for p in &req.paths[1..] {
        wb.add(p);
    }

    wb.hidden(!req.include_hidden);
    wb.git_ignore(req.respect_gitignore);
    wb.git_global(req.respect_gitignore);
    wb.git_exclude(req.respect_gitignore);
    wb.parents(req.respect_gitignore);
    wb.ignore(true);

    if let Some(max) = req.max_file_size_bytes {
        wb.max_filesize(Some(max));
    }

    if !req.file_types.is_empty() {
        let mut tb = TypesBuilder::new();
        tb.add_defaults();
        for t in &req.file_types {
            tb.select(t);
        }
        let types = tb.build()
            .map_err(|e| RipgrepError::Io(format!("type filter error: {e}")))?;
        wb.types(types);
    }

    if !req.include_globs.is_empty() || !req.exclude_globs.is_empty() {
        let mut ob = OverrideBuilder::new(first);
        for g in &req.include_globs {
            ob.add(g)
                .map_err(|e| RipgrepError::Io(format!("glob error: {e}")))?;
        }
        for g in &req.exclude_globs {
            ob.add(&format!("!{g}"))
                .map_err(|e| RipgrepError::Io(format!("glob error: {e}")))?;
        }
        let overrides = ob.build()
            .map_err(|e| RipgrepError::Io(format!("override error: {e}")))?;
        wb.overrides(overrides);
    }

    Ok(wb)
}
