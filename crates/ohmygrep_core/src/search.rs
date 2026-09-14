use crate::cancel::CancelToken;
use crate::error::OhMyGrepError;
use crate::options::{SearchMatch, SearchRequest, SearchResult, SearchSummary};
use crate::shared::{Limits, Shared, StoppableReader};
use crate::sink::{ChannelSink, SinkConfig};
use crossbeam_channel::{unbounded, Sender};
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, SearcherBuilder};
use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use ignore::{DirEntry, WalkBuilder, WalkState};
use std::io::Read;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::atomic::Ordering;
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

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

pub fn panic_safe<F, R>(f: F) -> Result<R, OhMyGrepError>
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    catch_unwind(f).map_err(|payload| OhMyGrepError::InternalPanic(panic_message(&*payload)))
}

#[cfg(test)]
pub fn force_panic_for_test() -> Result<(), OhMyGrepError> {
    panic_safe(|| panic!("intentional test panic"))
}

pub fn search_blocking(
    req: SearchRequest,
    cancel: Arc<CancelToken>,
) -> Result<SearchResult, OhMyGrepError> {
    search_with_limits(req, cancel, Limits::default())
}

pub(crate) fn search_with_limits(
    req: SearchRequest,
    cancel: Arc<CancelToken>,
    limits: Limits,
) -> Result<SearchResult, OhMyGrepError> {
    panic_safe(AssertUnwindSafe(move || {
        let prepared = prepare(req, limits)?;
        let (tx, rx) = unbounded();
        let summary = prepared.run(cancel, tx)?;
        let mut matches: Vec<SearchMatch> = rx.try_iter().collect();
        matches.sort_by(|a, b| {
            (a.path.as_str(), a.line_number).cmp(&(b.path.as_str(), b.line_number))
        });
        Ok(SearchResult {
            matches,
            truncated: summary.truncated,
            cancelled: summary.cancelled,
            files_searched: summary.files_searched,
            elapsed_ms: summary.elapsed_ms,
            warnings: summary.warnings,
        })
    }))?
}

/// A validated request, ready to run on the calling thread.
pub(crate) struct Prepared {
    req: SearchRequest,
    matcher: RegexMatcher,
    walker: WalkBuilder,
    limits: Limits,
}

/// Validates the request; every request-level error surfaces here, before any walk.
pub(crate) fn prepare(req: SearchRequest, limits: Limits) -> Result<Prepared, OhMyGrepError> {
    let matcher = build_matcher(&req)?;
    let mut walker = build_walker(&req)?;
    walker.threads(limits.threads);
    Ok(Prepared {
        req,
        matcher,
        walker,
        limits,
    })
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

impl Prepared {
    /// Walks and searches, sending matches to `tx`; returns once every worker is done.
    pub fn run(
        self,
        cancel: Arc<CancelToken>,
        tx: Sender<SearchMatch>,
    ) -> Result<SearchSummary, OhMyGrepError> {
        let start = Instant::now();
        let max_matches = self.req.max_matches.map(|n| n as usize);
        let max_files = self.req.max_files.map(|n| n as usize);
        let shared = Arc::new(Shared::new(cancel, max_matches, self.limits));
        let (req, matcher) = (&self.req, &self.matcher);

        self.walker.build_parallel().run(|| {
            let tx = tx.clone();
            let shared = Arc::clone(&shared);
            Box::new(move |entry| {
                if shared.should_stop() {
                    return WalkState::Quit;
                }
                if max_files.is_some_and(|max| shared.files.load(Ordering::Relaxed) >= max) {
                    return WalkState::Quit;
                }
                let entry = match entry {
                    Ok(e) => e,
                    Err(err) => {
                        shared.warnings.push_walk_error(&err);
                        return WalkState::Continue;
                    }
                };
                // Ignore-file syntax errors arrive attached to an otherwise valid entry.
                if let Some(err) = entry.error() {
                    shared.warnings.push_walk_error(err);
                }
                if !entry.file_type().is_some_and(|t| t.is_file()) {
                    return WalkState::Continue;
                }
                shared.files.fetch_add(1, Ordering::Relaxed);
                // A panic here must not kill the worker: ignore's parallel walker
                // would then wait forever for it to finish.
                let searched = catch_unwind(AssertUnwindSafe(|| {
                    search_file(req, matcher, &entry, &shared, &tx)
                }));
                match searched {
                    Ok(()) => WalkState::Continue,
                    Err(payload) => {
                        shared.record_panic(panic_message(&*payload));
                        WalkState::Quit
                    }
                }
            })
        });
        drop(tx);

        if let Some(message) = shared.take_panic() {
            return Err(OhMyGrepError::InternalPanic(message));
        }
        Ok(SearchSummary {
            truncated: shared.truncated(),
            cancelled: shared.cancel.is_cancelled(),
            files_searched: shared.files.load(Ordering::Relaxed) as u64,
            elapsed_ms: start.elapsed().as_millis() as u64,
            warnings: shared.warnings.take(),
        })
    }
}

fn search_file(
    req: &SearchRequest,
    matcher: &RegexMatcher,
    entry: &DirEntry,
    shared: &Arc<Shared>,
    tx: &Sender<SearchMatch>,
) {
    let explicit = entry.depth() == 0;
    let path = entry.path().to_string_lossy().into_owned();
    let mut sink = ChannelSink::new(
        path.clone(),
        tx.clone(),
        Arc::clone(shared),
        matcher.clone(),
        SinkConfig {
            before_context: req.before_context as usize,
            after_context: req.after_context as usize,
            max_columns: req.max_columns.map(|n| n as usize),
            explicit,
        },
    );
    let mut builder = SearcherBuilder::new();
    builder
        .line_number(true)
        .before_context(req.before_context as usize)
        .after_context(req.after_context as usize)
        .multi_line(req.multiline)
        .binary_detection(binary_detection(req, explicit));
    let stop_on_limit = req.after_context == 0;

    let file = match std::fs::File::open(entry.path()) {
        Ok(f) => f,
        Err(err) => return shared.warnings.push(path, err.to_string()),
    };
    let result = if req.multiline {
        match read_for_multiline(file, shared, stop_on_limit) {
            Ok(Some((buf, _guards))) => {
                // A BOM makes grep-searcher decode into a second buffer; cap it at
                // the extra budget reserved for it.
                builder.heap_limit(Some(DECODE_FACTOR * buf.len() + DECODE_SLACK));
                builder.build().search_slice(matcher, &buf, &mut sink)
            }
            Ok(None) => return,
            Err(message) => return shared.warnings.push(path, message),
        }
    } else {
        builder.heap_limit(Some(shared.limits.line_heap));
        builder.build().search_reader(
            matcher,
            StoppableReader::new(file, Arc::clone(shared), stop_on_limit),
            &mut sink,
        )
    };
    if let Err(err) = result {
        sink.finish_after_error(&err.to_string());
    }
}

/// Reads a whole file under the shared multiline budget. `Ok(None)` means the
/// search stopped while waiting for budget.
/// Budget reservations held for as long as a multiline buffer is alive.
type BudgetGuards = Vec<crate::shared::BudgetGuard>;

/// Decoding a BOM-prefixed buffer can take up to this many times its size.
const DECODE_FACTOR: usize = 3;
const DECODE_SLACK: usize = 64 * 1024;

fn has_bom(buf: &[u8]) -> bool {
    buf.starts_with(&[0xEF, 0xBB, 0xBF])
        || buf.starts_with(&[0xFF, 0xFE])
        || buf.starts_with(&[0xFE, 0xFF])
}

fn read_for_multiline(
    file: std::fs::File,
    shared: &Arc<Shared>,
    stop_on_limit: bool,
) -> Result<Option<(Vec<u8>, BudgetGuards)>, String> {
    let budget = shared.limits.multiline_budget;
    let too_big = || {
        format!(
            "skipped: file needs more than {} MiB for multiline search",
            budget >> 20
        )
    };
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    let Ok(len) = usize::try_from(len) else {
        return Err(too_big());
    };
    if len > budget {
        return Err(too_big());
    }
    let Some(guard) = shared.reserve_multiline(len) else {
        return Ok(None);
    };
    let mut buf = Vec::with_capacity(len);
    StoppableReader::new(file, Arc::clone(shared), stop_on_limit)
        .take(len as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    if buf.len() > len {
        return Err("skipped: file grew while being read".into());
    }
    let mut guards = vec![guard];
    if has_bom(&buf) {
        let extra = DECODE_FACTOR * len + DECODE_SLACK;
        if len + extra > budget {
            return Err(too_big());
        }
        match shared.reserve_multiline(extra) {
            Some(guard) => guards.push(guard),
            None => return Ok(None),
        }
    }
    Ok(Some((buf, guards)))
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
