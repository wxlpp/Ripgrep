use crate::cancel::CancelToken;
use crate::error::OhMyGrepError;
use crate::options::{SearchMatch, SearchRequest, SearchResult};
use crate::sink::ChannelSink;
use crate::warnings::Warnings;
use crossbeam_channel::unbounded;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, SearcherBuilder};
use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use ignore::{WalkBuilder, WalkState};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub fn build_matcher(req: &SearchRequest) -> Result<RegexMatcher, OhMyGrepError> {
    let mut b = RegexMatcherBuilder::new();
    b.case_insensitive(req.case_insensitive);
    b.case_smart(req.smart_case);
    b.multi_line(req.multiline);
    b.build(&req.pattern)
        .map_err(|e| OhMyGrepError::InvalidPattern(e.to_string()))
}

pub fn panic_safe<F, R>(f: F) -> Result<R, OhMyGrepError>
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
            Err(OhMyGrepError::InternalPanic(msg))
        }
    }
}

#[cfg(test)]
pub fn force_panic_for_test() -> Result<(), OhMyGrepError> {
    panic_safe(|| panic!("intentional test panic"))
}

pub fn search_blocking(
    req: SearchRequest,
    cancel: Arc<CancelToken>,
) -> Result<SearchResult, OhMyGrepError> {
    panic_safe(move || search_blocking_inner(req, cancel))?
}

/// rg's policy: files reached by walking stop at the first NUL; files named
/// explicitly are searched with NULs treated as line breaks so a match is
/// still reported (as a warning, see `ChannelSink::finish`).
fn binary_detection(req: &SearchRequest, explicit: bool) -> BinaryDetection {
    if req.search_binary {
        BinaryDetection::none()
    } else if explicit {
        BinaryDetection::convert(b'\x00')
    } else {
        BinaryDetection::quit(b'\x00')
    }
}

fn search_blocking_inner(
    req: SearchRequest,
    cancel: Arc<CancelToken>,
) -> Result<SearchResult, OhMyGrepError> {
    let start = Instant::now();

    let matcher = build_matcher(&req)?;
    let walker = build_walker(&req)?.build_parallel();

    let (tx, rx) = unbounded::<SearchMatch>();
    // Counters only bound the walk; parallel workers may overshoot a limit, and
    // the truncation after collection is what makes results exact. Stronger
    // orderings would not remove that check-then-act race.
    let match_counter = Arc::new(AtomicUsize::new(0));
    let file_counter = Arc::new(AtomicUsize::new(0));
    let warnings = Arc::new(Warnings::default());

    let max_matches = req.max_matches.map(|n| n as usize);
    let max_files = req.max_files.map(|n| n as usize);
    let before = req.before_context as usize;
    let after = req.after_context as usize;

    walker.run(|| {
        let tx = tx.clone();
        let cancel = Arc::clone(&cancel);
        let match_counter = Arc::clone(&match_counter);
        let file_counter = Arc::clone(&file_counter);
        let warnings = Arc::clone(&warnings);
        let matcher = matcher.clone();
        let req = &req;
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
                Err(err) => {
                    warnings.push_walk_error(&err);
                    return WalkState::Continue;
                }
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                return WalkState::Continue;
            }
            file_counter.fetch_add(1, Ordering::Relaxed);

            let explicit = entry.depth() == 0;
            let path = entry.path().to_string_lossy().into_owned();
            let mut sink = ChannelSink::new(
                path.clone(),
                tx.clone(),
                Arc::clone(&cancel),
                Arc::clone(&match_counter),
                matcher.clone(),
                before,
            )
            .reporting_to(Arc::clone(&warnings), explicit);
            let mut sb = SearcherBuilder::new();
            sb.line_number(true);
            sb.before_context(before);
            sb.after_context(after);
            sb.multi_line(req.multiline);
            sb.binary_detection(binary_detection(req, explicit));
            if let Err(err) = sb.build().search_path(&matcher, entry.path(), &mut sink) {
                warnings.push(path, err.to_string());
            }
            WalkState::Continue
        })
    });

    drop(tx);
    let mut matches: Vec<SearchMatch> = rx.iter().collect();
    matches.sort_by(|a, b| (a.path.as_str(), a.line_number).cmp(&(b.path.as_str(), b.line_number)));

    // `>=`: collecting exactly `max` means the walk was stopped by the limit.
    let truncated = match max_matches {
        Some(max) if matches.len() >= max => {
            matches.truncate(max);
            true
        }
        _ => false,
    };

    let mut warnings = warnings.take();
    warnings.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(SearchResult {
        matches,
        truncated,
        cancelled: cancel.is_cancelled(),
        files_searched: file_counter.load(Ordering::Relaxed) as u64,
        elapsed_ms: start.elapsed().as_millis() as u64,
        warnings,
    })
}

pub fn build_walker(req: &SearchRequest) -> Result<WalkBuilder, OhMyGrepError> {
    let Some(first) = req.paths.first() else {
        return Err(OhMyGrepError::InvalidArguments(
            "at least one path is required".into(),
        ));
    };
    for p in &req.paths {
        if !Path::new(p).exists() {
            return Err(OhMyGrepError::PathNotFound(p.clone()));
        }
    }

    let mut wb = WalkBuilder::new(first);
    for p in &req.paths[1..] {
        wb.add(p);
    }

    let respect = req.respect_gitignore;
    wb.hidden(!req.include_hidden);
    wb.git_ignore(respect);
    wb.git_global(respect);
    wb.git_exclude(respect);
    wb.parents(respect);
    wb.ignore(respect);
    wb.require_git(req.require_git);
    if respect {
        wb.add_custom_ignore_filename(".rgignore");
    }

    if let Some(max) = req.max_file_size_bytes {
        wb.max_filesize(Some(max));
    }

    let invalid = |what: &str, e: &dyn std::fmt::Display| {
        OhMyGrepError::InvalidArguments(format!("{what}: {e}"))
    };

    if !req.file_types.is_empty() {
        let mut tb = TypesBuilder::new();
        tb.add_defaults();
        for t in &req.file_types {
            tb.select(t);
        }
        let types = tb.build().map_err(|e| invalid("file type", &e))?;
        wb.types(types);
    }

    if !req.include_globs.is_empty() || !req.exclude_globs.is_empty() {
        let mut ob = OverrideBuilder::new(first);
        for g in &req.include_globs {
            ob.add(g).map_err(|e| invalid("glob", &e))?;
        }
        for g in &req.exclude_globs {
            ob.add(&format!("!{g}")).map_err(|e| invalid("glob", &e))?;
        }
        let overrides = ob.build().map_err(|e| invalid("glob", &e))?;
        wb.overrides(overrides);
    }

    Ok(wb)
}
