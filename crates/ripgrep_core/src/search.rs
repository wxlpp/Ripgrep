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
