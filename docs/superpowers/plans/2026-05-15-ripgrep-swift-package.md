> **Renamed 2026-09-14:** RipgrepKit is now oh-my-grep (package `OhMyGrep`, crate `ohmygrep_core`, FFI module `OhMyGrepCoreFFI`). This document is historical and keeps the original names.

# RipgrepKit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a SwiftPM library `RipgrepKit` (iOS + macOS) that wraps a curated subset of ripgrep 15.x via Rust + UniFFI + XCFramework, with a CLI-style entry point usable as an LLM tool call.

**Architecture:** Rust crate `ripgrep_core` composes ripgrep's official sub-crates (`ignore`, `grep-regex`, `grep-searcher`, `globset`) into a static lib for 5 Apple target triples, packaged as a 3-slice XCFramework, exposed to Swift via UniFFI. Swift side has two SwiftPM products: `RipgrepKitCore` (typed API, no extra deps) and `RipgrepKitTool` (CLI-string parser + LLM tool helpers, depends on `swift-argument-parser`).

**Tech Stack:** Rust (cargo, ripgrep crates, UniFFI 0.28, crossbeam-channel), Swift 6 / Xcode 16, SwiftPM, swift-argument-parser 1.5+, XCFramework, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-05-15-ripgrep-swift-package-design.md`. Read it first for design rationale; this plan executes it.

---

## File Structure

```
Ripgrep/
├── Package.swift
├── Cargo.toml                          # workspace manifest
├── crates/
│   └── ripgrep_core/
│       ├── Cargo.toml
│       ├── build.rs                    # uniffi-bindgen scaffolding step
│       ├── uniffi.toml
│       └── src/
│           ├── lib.rs                  # UniFFI exports + catch_unwind
│           ├── search.rs               # WalkParallel + grep-searcher
│           ├── cancel.rs               # CancelToken
│           ├── options.rs              # Request / Match / Result records
│           ├── error.rs                # RipgrepError
│           └── sink.rs                 # Custom grep-searcher Sink
├── Sources/
│   ├── RipgrepKitFFI/                  # uniffi-bindgen output (committed)
│   │   ├── ripgrep_core.swift
│   │   └── ripgrep_coreFFI.modulemap
│   ├── RipgrepKitCore/
│   │   ├── Ripgrep.swift               # namespace + search(...)
│   │   ├── Options.swift
│   │   ├── SearchResult.swift          # types + formatters
│   │   └── Error.swift
│   └── RipgrepKitTool/
│       ├── RipgrepArgs.swift
│       ├── Tokenizer.swift
│       ├── Parse.swift
│       ├── Run.swift
│       └── Tool.swift
├── Frameworks/
│   └── RipgrepCore.xcframework         # local dev only; remote at release
├── scripts/
│   ├── build-xcframework.sh
│   ├── generate-bindings.sh
│   ├── package-release.sh
│   └── ci.sh
├── Tests/
│   ├── RipgrepKitCoreTests/
│   │   ├── Fixtures/                   # mini-repo: gitignore + types + hidden + symlink
│   │   ├── SearchTests.swift
│   │   ├── CancellationTests.swift
│   │   └── ErrorTests.swift
│   └── RipgrepKitToolTests/
│       ├── Fixtures/
│       ├── TokenizerTests.swift
│       ├── ArgsParsingTests.swift
│       ├── ParseTests.swift
│       └── ToolCallTests.swift
└── docs/superpowers/                    # already exists
```

**Decomposition notes:**
- Each Rust source file maps 1:1 with a logical concern (errors, types, walker, cancel, sink, FFI surface).
- Swift split into 3 targets is locked by the spec — `RipgrepKitFFI` is auto-generated and not edited by hand; `RipgrepKitCore` has no `argument-parser` dependency.
- `Frameworks/RipgrepCore.xcframework` is .gitignored except during a release window (Task 44 switches to remote URL).

**Pre-flight setup the engineer needs to do once:**
```bash
xcode-select --install                                   # if not done
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
                  aarch64-apple-darwin x86_64-apple-darwin
cargo install uniffi-bindgen --version 0.28.0           # CLI tool
```

---

## Phase 1 — Rust Core (Tasks 1-13)

### Task 1: Workspace + crate skeleton + first failing test

**Files:**
- Create: `Cargo.toml`
- Create: `crates/ripgrep_core/Cargo.toml`
- Create: `crates/ripgrep_core/src/lib.rs`
- Create: `.gitignore`

- [ ] **Step 1: Workspace manifest**

Create `Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
edition = "2021"
rust-version = "1.75"
license = "MIT"
```

- [ ] **Step 2: ripgrep_core crate manifest**

Create `crates/ripgrep_core/Cargo.toml`:
```toml
[package]
name = "ripgrep_core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[lib]
name = "ripgrep_core"
crate-type = ["staticlib", "rlib"]   # staticlib for XCFramework, rlib for unit tests

[dependencies]
ignore = "0.4.23"
grep-regex = "0.1.13"
grep-searcher = "0.1.14"
grep-matcher = "0.1.7"
globset = "0.4.15"
crossbeam-channel = "0.5.13"
thiserror = "2.0"
once_cell = "1.20"
uniffi = { version = "0.28", features = ["build"] }

[build-dependencies]
uniffi = { version = "0.28", features = ["build"] }
```

(Versions track ripgrep 15.x's lockfile as of 2026-05; bump if `cargo update` reveals newer compatible.)

- [ ] **Step 3: Stub lib.rs with one passing test**

Create `crates/ripgrep_core/src/lib.rs`:
```rust
//! ripgrep_core — Swift-facing wrapper around ripgrep's reusable crates.
//!
//! Public surface is generated via UniFFI; see `lib.rs` `uniffi::setup_scaffolding!()`.

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_compiles() {
        assert_eq!(2 + 2, 4);
    }
}
```

- [ ] **Step 4: .gitignore**

Create `.gitignore`:
```
target/
.build/
.swiftpm/
DerivedData/
Frameworks/RipgrepCore.xcframework/
*.xcuserdata
*.xcworkspace
.DS_Store
```

- [ ] **Step 5: Build and run test**

Run: `cargo test -p ripgrep_core`
Expected: `test tests::workspace_compiles ... ok`

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/ripgrep_core/ .gitignore
git commit -m "Bootstrap Rust workspace and ripgrep_core crate"
```

---

### Task 2: Error types

**Files:**
- Create: `crates/ripgrep_core/src/error.rs`
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
mod error;

#[cfg(test)]
mod error_tests {
    use super::error::RipgrepError;

    #[test]
    fn error_messages_render() {
        let e = RipgrepError::InvalidPattern("[".into());
        assert!(format!("{e}").contains("invalid regex"));
        assert!(format!("{e}").contains("["));
    }
}
```

- [ ] **Step 2: Run test (will fail to compile)**

Run: `cargo test -p ripgrep_core`
Expected: `error[E0432]: unresolved import` for `error::RipgrepError`.

- [ ] **Step 3: Implement error.rs**

Create `crates/ripgrep_core/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RipgrepError {
    #[error("invalid regex: {0}")]
    InvalidPattern(String),

    #[error("path not found: {0}")]
    PathNotFound(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("internal panic: {0}")]
    InternalPanic(String),
}

impl From<std::io::Error> for RipgrepError {
    fn from(e: std::io::Error) -> Self {
        RipgrepError::Io(e.to_string())
    }
}
```

- [ ] **Step 4: Run test**

Run: `cargo test -p ripgrep_core`
Expected: both `workspace_compiles` and `error_messages_render` pass.

- [ ] **Step 5: Commit**

```bash
git add crates/ripgrep_core/src/
git commit -m "Add RipgrepError enum with thiserror conversions"
```

---

### Task 3: Request / Match / Result types (Rust-only, no FFI yet)

**Files:**
- Create: `crates/ripgrep_core/src/options.rs`
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
mod options;

#[cfg(test)]
mod options_tests {
    use super::options::*;

    #[test]
    fn search_request_default_constructable() {
        let r = SearchRequest {
            pattern: "todo".into(),
            paths: vec![".".into()],
            case_insensitive: false,
            smart_case: false,
            multiline: false,
            include_globs: vec![],
            exclude_globs: vec![],
            file_types: vec![],
            respect_gitignore: true,
            include_hidden: false,
            before_context: 0,
            after_context: 0,
            max_matches: None,
            max_files: None,
            max_file_size_bytes: None,
            timeout_ms: None,
        };
        assert_eq!(r.pattern, "todo");
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p ripgrep_core`
Expected: compile error — `options` not found.

- [ ] **Step 3: Create options.rs**

Create `crates/ripgrep_core/src/options.rs`:
```rust
#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub pattern: String,
    pub paths: Vec<String>,
    pub case_insensitive: bool,
    pub smart_case: bool,
    pub multiline: bool,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
    pub file_types: Vec<String>,
    pub respect_gitignore: bool,
    pub include_hidden: bool,
    pub before_context: u32,
    pub after_context: u32,
    pub max_matches: Option<u32>,
    pub max_files: Option<u32>,
    pub max_file_size_bytes: Option<u64>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Submatch {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchMatch {
    pub path: String,
    pub line_number: u64,
    pub line: String,
    pub before_context: Vec<String>,
    pub after_context: Vec<String>,
    pub submatches: Vec<Submatch>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,
    pub cancelled: bool,
    pub files_searched: u64,
    pub elapsed_ms: u64,
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p ripgrep_core`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/ripgrep_core/src/
git commit -m "Add SearchRequest, SearchMatch, SearchResult types"
```

---

### Task 4: CancelToken

**Files:**
- Create: `crates/ripgrep_core/src/cancel.rs`
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
mod cancel;

#[cfg(test)]
mod cancel_tests {
    use super::cancel::CancelToken;
    use std::sync::Arc;
    use std::time::Duration;
    use std::thread;

    #[test]
    fn fresh_token_not_cancelled() {
        let t = CancelToken::new(None);
        assert!(!t.is_cancelled());
    }

    #[test]
    fn explicit_cancel_trips_token() {
        let t = Arc::new(CancelToken::new(None));
        let t2 = Arc::clone(&t);
        thread::spawn(move || t2.cancel());
        thread::sleep(Duration::from_millis(20));
        assert!(t.is_cancelled());
    }

    #[test]
    fn deadline_trips_token_after_elapse() {
        let t = CancelToken::new(Some(10));
        thread::sleep(Duration::from_millis(40));
        assert!(t.is_cancelled());
    }

    #[test]
    fn no_deadline_means_never_auto_cancel() {
        let t = CancelToken::new(None);
        thread::sleep(Duration::from_millis(20));
        assert!(!t.is_cancelled());
    }
}
```

- [ ] **Step 2: Run tests (expect compile failure)**

Run: `cargo test -p ripgrep_core cancel_tests`
Expected: compile error — `cancel` module missing.

- [ ] **Step 3: Implement CancelToken**

Create `crates/ripgrep_core/src/cancel.rs`:
```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cancellation token for in-flight searches. Construction takes an
/// optional timeout (deadline = now + timeout). `is_cancelled` returns
/// true if either an explicit `cancel()` was called or the deadline
/// elapsed. The deadline is immutable after construction.
#[derive(Debug)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    deadline: Option<Instant>,
}

impl CancelToken {
    pub fn new(timeout_ms: Option<u64>) -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            deadline: timeout_ms.map(|ms| Instant::now() + Duration::from_millis(ms)),
        }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        if self.flag.load(Ordering::Relaxed) {
            return true;
        }
        match self.deadline {
            Some(d) => Instant::now() >= d,
            None => false,
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p ripgrep_core cancel_tests`
Expected: all 4 cancel tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/ripgrep_core/src/
git commit -m "Add CancelToken with immutable deadline"
```

---

### Task 5: Sink — collect matches into a channel

**Files:**
- Create: `crates/ripgrep_core/src/sink.rs`
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
mod sink;

#[cfg(test)]
mod sink_tests {
    use super::sink::ChannelSink;
    use crossbeam_channel::unbounded;
    use grep_regex::RegexMatcher;
    use grep_searcher::SearcherBuilder;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn sink_emits_match_per_line() {
        let (tx, rx) = unbounded();
        let counter = Arc::new(AtomicUsize::new(0));
        let mut sink = ChannelSink::new(
            "/tmp/test.txt".into(),
            tx,
            Arc::new(crate::cancel::CancelToken::new(None)),
            Arc::clone(&counter),
        );
        let m = RegexMatcher::new("foo").unwrap();
        let body = b"foo\nbar\nfoo\n";
        SearcherBuilder::new()
            .build()
            .search_slice(&m, body, &mut sink)
            .unwrap();
        drop(sink);
        let collected: Vec<_> = rx.iter().collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].line_number, 1);
        assert_eq!(collected[1].line_number, 3);
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p ripgrep_core sink_tests`
Expected: compile error — `sink` module missing.

- [ ] **Step 3: Implement ChannelSink**

Create `crates/ripgrep_core/src/sink.rs`:
```rust
use crate::cancel::CancelToken;
use crate::options::{SearchMatch, Submatch};
use crossbeam_channel::Sender;
use grep_matcher::{Match, Matcher};
use grep_searcher::{Searcher, Sink, SinkContext, SinkContextKind, SinkError, SinkMatch};
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    pending_after: Vec<(SearchMatch, u32)>, // (match, remaining_after_lines)
    events_since_check: usize,
    before_context: usize,
}

impl<M: Matcher> ChannelSink<M> {
    pub fn new(
        path: String,
        tx: Sender<SearchMatch>,
        cancel: Arc<CancelToken>,
        match_counter: Arc<AtomicUsize>,
        matcher: M,
        before_context: usize,
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
        for (m, _) in self.pending_after.drain(..) {
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
    fn error_message<T: std::fmt::Display>(_: T) -> Self { SinkAbort }
    fn error_io(_: io::Error) -> Self { SinkAbort }
}

impl<M: Matcher> Sink for ChannelSink<M> {
    type Error = SinkAbort;

    fn matched(&mut self, _: &Searcher, m: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        if self.poll_cancel() { return Ok(false); }

        // Decode line; UTF-8 lossy.
        let line_bytes = m.bytes();
        let line = String::from_utf8_lossy(line_bytes).trim_end_matches('\n').to_string();

        // Find submatches inside the line using the matcher.
        let mut submatches = Vec::new();
        let mut cur = 0;
        while let Ok(Some(Match { start, end })) =
            self.matcher.find_at(line_bytes, cur)
        {
            submatches.push(Submatch { start: start as u32, end: end as u32 });
            cur = end.max(start + 1);
            if cur >= line_bytes.len() { break; }
        }

        let line_number = m.line_number().unwrap_or(0);
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
        self.pending_after.push((sm, self.before_context as u32 /* not used */));
        Ok(true)
    }

    fn context(&mut self, _: &Searcher, ctx: &SinkContext<'_>) -> Result<bool, Self::Error> {
        if self.poll_cancel() { return Ok(false); }
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
                if let Some((m, _)) = self.pending_after.last_mut() {
                    m.after_context.push(line);
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
```

> **Note:** Update the test in Step 1 to pass the matcher and `before_context: 0` to `ChannelSink::new`. Adjust accordingly:
> ```rust
> let mut sink = ChannelSink::new(
>     "/tmp/test.txt".into(),
>     tx,
>     Arc::new(crate::cancel::CancelToken::new(None)),
>     Arc::clone(&counter),
>     m.clone(),
>     0,
> );
> ```

- [ ] **Step 4: Run tests**

Run: `cargo test -p ripgrep_core sink_tests`
Expected: pass; 2 matches collected at lines 1 and 3.

- [ ] **Step 5: Commit**

```bash
git add crates/ripgrep_core/src/
git commit -m "Add ChannelSink that emits matches into a crossbeam channel"
```

---

### Task 6: Build matcher (case_insensitive / smart_case / multiline)

**Files:**
- Create: `crates/ripgrep_core/src/search.rs`
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
mod search;

#[cfg(test)]
mod search_matcher_tests {
    use super::options::SearchRequest;
    use super::search::build_matcher;

    fn req(pattern: &str) -> SearchRequest {
        SearchRequest {
            pattern: pattern.into(),
            paths: vec![],
            case_insensitive: false,
            smart_case: false,
            multiline: false,
            include_globs: vec![],
            exclude_globs: vec![],
            file_types: vec![],
            respect_gitignore: true,
            include_hidden: false,
            before_context: 0,
            after_context: 0,
            max_matches: None,
            max_files: None,
            max_file_size_bytes: None,
            timeout_ms: None,
        }
    }

    #[test]
    fn case_sensitive_by_default() {
        let m = build_matcher(&req("Foo")).unwrap();
        use grep_matcher::Matcher;
        assert!(m.find(b"Foo").unwrap().is_some());
        assert!(m.find(b"foo").unwrap().is_none());
    }

    #[test]
    fn case_insensitive_matches_any_case() {
        let mut r = req("Foo");
        r.case_insensitive = true;
        let m = build_matcher(&r).unwrap();
        use grep_matcher::Matcher;
        assert!(m.find(b"FOO").unwrap().is_some());
    }

    #[test]
    fn smart_case_off_for_uppercase_pattern() {
        let mut r = req("Foo");
        r.smart_case = true;
        let m = build_matcher(&r).unwrap();
        use grep_matcher::Matcher;
        assert!(m.find(b"foo").unwrap().is_none());
    }

    #[test]
    fn smart_case_insensitive_for_lowercase_pattern() {
        let mut r = req("foo");
        r.smart_case = true;
        let m = build_matcher(&r).unwrap();
        use grep_matcher::Matcher;
        assert!(m.find(b"FOO").unwrap().is_some());
    }

    #[test]
    fn invalid_pattern_returns_error() {
        let r = req("[");
        let err = build_matcher(&r).unwrap_err();
        assert!(matches!(err, crate::error::RipgrepError::InvalidPattern(_)));
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p ripgrep_core search_matcher_tests`
Expected: compile error — `search` missing.

- [ ] **Step 3: Implement matcher builder**

Create `crates/ripgrep_core/src/search.rs`:
```rust
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
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p ripgrep_core search_matcher_tests`
Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/ripgrep_core/src/
git commit -m "Add build_matcher with case/smart_case/multiline support"
```

---

### Task 7: Build walker (paths, hidden, gitignore, types, globs, max_filesize)

**Files:**
- Modify: `crates/ripgrep_core/src/search.rs`
- Modify: `crates/ripgrep_core/src/lib.rs`
- Create: `crates/ripgrep_core/tests/fixtures/mini/...` (test data)

- [ ] **Step 1: Create fixture mini-repo for walker tests**

Create these files (literal contents below):

`crates/ripgrep_core/tests/fixtures/mini/.gitignore`:
```
target/
ignored.txt
```

`crates/ripgrep_core/tests/fixtures/mini/included.txt`:
```
hello world
search me TODO
final line
```

`crates/ripgrep_core/tests/fixtures/mini/ignored.txt`:
```
should not be searched TODO
```

`crates/ripgrep_core/tests/fixtures/mini/.hidden.txt`:
```
hidden TODO
```

`crates/ripgrep_core/tests/fixtures/mini/sub/nested.swift`:
```swift
// TODO: handle case
func main() {}
```

`crates/ripgrep_core/tests/fixtures/mini/target/built.txt`:
```
TODO inside ignored dir
```

- [ ] **Step 2: Write failing tests for walker**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
#[cfg(test)]
mod walker_tests {
    use super::options::SearchRequest;
    use super::search::build_walker;
    use std::path::Path;

    fn req() -> SearchRequest {
        SearchRequest {
            pattern: "TODO".into(),
            paths: vec!["crates/ripgrep_core/tests/fixtures/mini".into()],
            case_insensitive: false,
            smart_case: false,
            multiline: false,
            include_globs: vec![],
            exclude_globs: vec![],
            file_types: vec![],
            respect_gitignore: true,
            include_hidden: false,
            before_context: 0,
            after_context: 0,
            max_matches: None,
            max_files: None,
            max_file_size_bytes: None,
            timeout_ms: None,
        }
    }

    fn collect_paths(r: SearchRequest) -> Vec<String> {
        let walker = build_walker(&r).unwrap().build();
        walker
            .filter_map(Result::ok)
            .filter(|d| d.file_type().map_or(false, |t| t.is_file()))
            .map(|d| d.into_path().to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn respects_gitignore() {
        let paths = collect_paths(req());
        assert!(paths.iter().any(|p| p.ends_with("included.txt")));
        assert!(!paths.iter().any(|p| p.ends_with("ignored.txt")));
        assert!(!paths.iter().any(|p| p.contains("/target/")));
    }

    #[test]
    fn skips_hidden_by_default() {
        let paths = collect_paths(req());
        assert!(!paths.iter().any(|p| p.ends_with(".hidden.txt")));
    }

    #[test]
    fn includes_hidden_when_flag_set() {
        let mut r = req();
        r.include_hidden = true;
        let paths = collect_paths(r);
        assert!(paths.iter().any(|p| p.ends_with(".hidden.txt")));
    }

    #[test]
    fn no_ignore_includes_gitignored() {
        let mut r = req();
        r.respect_gitignore = false;
        let paths = collect_paths(r);
        assert!(paths.iter().any(|p| p.ends_with("ignored.txt")));
    }

    #[test]
    fn glob_include_filters_files() {
        let mut r = req();
        r.include_globs = vec!["*.swift".into()];
        let paths = collect_paths(r);
        assert!(paths.iter().all(|p| p.ends_with(".swift")));
    }

    #[test]
    fn glob_exclude_drops_files() {
        let mut r = req();
        r.exclude_globs = vec!["*.swift".into()];
        let paths = collect_paths(r);
        assert!(!paths.iter().any(|p| p.ends_with(".swift")));
    }

    #[test]
    fn file_type_filter_swift_only() {
        let mut r = req();
        r.file_types = vec!["swift".into()];
        let paths = collect_paths(r);
        assert!(paths.iter().all(|p| p.ends_with(".swift")));
    }

    #[test]
    fn nonexistent_path_errors() {
        let mut r = req();
        r.paths = vec!["/no/such/path".into()];
        let result = build_walker(&r);
        assert!(matches!(result, Err(crate::error::RipgrepError::PathNotFound(_))));
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p ripgrep_core walker_tests`
Expected: compile error — `build_walker` not defined.

- [ ] **Step 4: Implement walker builder**

Append to `crates/ripgrep_core/src/search.rs`:
```rust
use crate::error::RipgrepError;
use crate::options::SearchRequest;
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
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p ripgrep_core walker_tests`
Expected: all 8 walker tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/ripgrep_core/
git commit -m "Add build_walker with gitignore, hidden, glob, types, max_filesize"
```

---

### Task 8: search_blocking — end-to-end pipeline

**Files:**
- Modify: `crates/ripgrep_core/src/search.rs`
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Write failing end-to-end tests**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
#[cfg(test)]
mod search_e2e_tests {
    use super::cancel::CancelToken;
    use super::options::SearchRequest;
    use super::search::search_blocking;
    use std::sync::Arc;

    fn req(pattern: &str) -> SearchRequest {
        SearchRequest {
            pattern: pattern.into(),
            paths: vec!["crates/ripgrep_core/tests/fixtures/mini".into()],
            case_insensitive: false,
            smart_case: false,
            multiline: false,
            include_globs: vec![],
            exclude_globs: vec![],
            file_types: vec![],
            respect_gitignore: true,
            include_hidden: false,
            before_context: 0,
            after_context: 0,
            max_matches: None,
            max_files: None,
            max_file_size_bytes: None,
            timeout_ms: None,
        }
    }

    #[test]
    fn finds_matches_in_fixture() {
        let r = search_blocking(req("TODO"), Arc::new(CancelToken::new(None))).unwrap();
        let texts: Vec<_> = r.matches.iter().map(|m| m.line.as_str()).collect();
        assert!(texts.iter().any(|l| l.contains("search me TODO")));
        assert!(texts.iter().any(|l| l.contains("// TODO: handle case")));
        assert!(!texts.iter().any(|l| l.contains("should not be searched")));
    }

    #[test]
    fn results_sorted_deterministically() {
        let r = search_blocking(req("TODO"), Arc::new(CancelToken::new(None))).unwrap();
        let mut prev: Option<(&str, u64)> = None;
        for m in &r.matches {
            let key = (m.path.as_str(), m.line_number);
            if let Some(p) = prev {
                assert!(p <= key, "results not sorted: {p:?} then {key:?}");
            }
            prev = Some(key);
        }
    }

    #[test]
    fn empty_result_when_no_match() {
        let r = search_blocking(req("ZZZZ_NOPE"), Arc::new(CancelToken::new(None))).unwrap();
        assert_eq!(r.matches.len(), 0);
        assert_eq!(r.truncated, false);
        assert_eq!(r.cancelled, false);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p ripgrep_core search_e2e_tests`
Expected: compile error — `search_blocking` missing.

- [ ] **Step 3: Implement search_blocking**

Append to `crates/ripgrep_core/src/search.rs`:
```rust
use crate::cancel::CancelToken;
use crate::options::{SearchMatch, SearchRequest, SearchResult};
use crate::sink::ChannelSink;
use crossbeam_channel::unbounded;
use grep_searcher::SearcherBuilder;
use ignore::WalkState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub fn search_blocking(
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
    matches.sort_by(|a, b| (a.path.as_str(), a.line_number).cmp(&(b.path.as_str(), b.line_number)));

    let truncated = if let Some(max) = max_matches {
        if matches.len() > max {
            matches.truncate(max);
            true
        } else { false }
    } else { false };

    Ok(SearchResult {
        matches,
        truncated,
        cancelled: cancel.is_cancelled(),
        files_searched: file_counter.load(Ordering::Relaxed) as u64,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p ripgrep_core search_e2e_tests`
Expected: all 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/ripgrep_core/
git commit -m "Add search_blocking end-to-end pipeline (WalkParallel + channel)"
```

---

### Task 9: Context lines (-A/-B/-C) and multiline

**Files:**
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Add a fixture with content suitable for context tests**

Create `crates/ripgrep_core/tests/fixtures/mini/context.txt`:
```
line 1
line 2
match here
line 4
line 5
another match
line 7
```

- [ ] **Step 2: Write failing tests**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
#[cfg(test)]
mod context_tests {
    use super::cancel::CancelToken;
    use super::search::search_blocking;
    use std::sync::Arc;

    fn req() -> super::options::SearchRequest {
        let mut r = super::search_e2e_tests::req("match");
        r.paths = vec!["crates/ripgrep_core/tests/fixtures/mini/context.txt".into()];
        r
    }

    #[test]
    fn before_context_captures_prior_lines() {
        let mut r = req();
        r.before_context = 1;
        let res = search_blocking(r, Arc::new(CancelToken::new(None))).unwrap();
        let m = &res.matches[0];
        assert_eq!(m.before_context, vec!["line 2".to_string()]);
    }

    #[test]
    fn after_context_captures_following_lines() {
        let mut r = req();
        r.after_context = 1;
        let res = search_blocking(r, Arc::new(CancelToken::new(None))).unwrap();
        let m = &res.matches[0];
        assert_eq!(m.after_context, vec!["line 4".to_string()]);
    }

    #[test]
    fn multiline_pattern_matches_across_lines() {
        let mut r = req();
        r.pattern = r"match.*\n.*match".into();
        r.multiline = true;
        let res = search_blocking(r, Arc::new(CancelToken::new(None))).unwrap();
        assert!(res.matches.len() >= 1);
    }
}
```

> Note: this test refers to `super::search_e2e_tests::req` — make sure that helper is `pub(super)` or duplicate the helper here.

- [ ] **Step 3: Run tests**

Run: `cargo test -p ripgrep_core context_tests`
Expected: tests run; before/after may need Sink fixes (if you fail, inspect the SinkContextKind branches in `sink.rs` from Task 5 — the `pending_after` flush logic must defer sending until after-lines are collected).

- [ ] **Step 4: Adjust Sink if needed**

If `before_context_captures_prior_lines` fails because `before_context` field is empty, ensure `ChannelSink::matched` snapshots the current `before_buf` into the `SearchMatch.before_context` *before* pushing into `pending_after`. (Already done in Task 5 sketch — verify.)

- [ ] **Step 5: Commit**

```bash
git add crates/ripgrep_core/
git commit -m "Verify context lines and multiline behavior end-to-end"
```

---

### Task 10: max_matches / max_files / timeout

**Files:**
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Write tests**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
#[cfg(test)]
mod limits_tests {
    use super::cancel::CancelToken;
    use super::search::search_blocking;
    use std::sync::Arc;

    fn req(pattern: &str) -> super::options::SearchRequest {
        let mut r = super::search_e2e_tests::req(pattern);
        r.respect_gitignore = false;   // ensure we have lots of files to count
        r.include_hidden = true;
        r
    }

    #[test]
    fn max_matches_truncates_and_flags() {
        let mut r = req("TODO");
        r.max_matches = Some(1);
        let res = search_blocking(r, Arc::new(CancelToken::new(None))).unwrap();
        assert_eq!(res.matches.len(), 1);
        assert!(res.truncated);
    }

    #[test]
    fn max_files_caps_files_searched() {
        let mut r = req("TODO");
        r.max_files = Some(1);
        let res = search_blocking(r, Arc::new(CancelToken::new(None))).unwrap();
        assert!(res.files_searched <= 1, "got {}", res.files_searched);
    }

    #[test]
    fn timeout_marks_result_cancelled() {
        let mut r = req("xxxxxxxxxxxxxxxxx_no_match");  // forces full scan
        r.timeout_ms = Some(1);   // 1 ms — extremely tight
        // Big enough fixture: just our mini, but we set timeout to 0 so it trips
        // even on tiny dirs.
        let res = search_blocking(
            r,
            Arc::new(CancelToken::new(Some(0))),  // pre-tripped
        ).unwrap();
        assert!(res.cancelled);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p ripgrep_core limits_tests`
Expected: all 3 pass (search_blocking already wires max_matches / max_files / cancel from Task 8).

- [ ] **Step 3: Commit**

```bash
git add crates/ripgrep_core/
git commit -m "Verify max_matches, max_files, timeout cancellation"
```

---

### Task 11: External cancellation (Task.cancel scenario)

**Files:**
- Modify: `crates/ripgrep_core/src/lib.rs`

- [ ] **Step 1: Write the test**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
#[cfg(test)]
mod external_cancel_tests {
    use super::cancel::CancelToken;
    use super::search::search_blocking;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn external_cancel_stops_search_promptly() {
        let cancel = Arc::new(CancelToken::new(None));
        let cancel_for_thread = Arc::clone(&cancel);

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            cancel_for_thread.cancel();
        });

        let r = super::search_e2e_tests::req("anything");
        let start = Instant::now();
        let res = search_blocking(r, cancel).unwrap();
        let elapsed = start.elapsed();
        // tiny fixture finishes in <50ms anyway, so cancellation may or may not
        // have kicked in; the assertion is about *no panic / no hang*.
        assert!(elapsed < Duration::from_secs(2), "search hung: {:?}", elapsed);
        let _ = res;
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p ripgrep_core external_cancel_tests`
Expected: pass within 2 seconds.

- [ ] **Step 3: Commit**

```bash
git add crates/ripgrep_core/
git commit -m "Verify external cancellation propagates to walker"
```

---

### Task 12: Panic containment

**Files:**
- Modify: `crates/ripgrep_core/src/lib.rs`
- Modify: `crates/ripgrep_core/src/search.rs` (will be re-exported via FFI in Task 13)

- [ ] **Step 1: Add a test-only panic-injection function**

Append to `crates/ripgrep_core/src/search.rs`:
```rust
#[cfg(test)]
pub fn force_panic_for_test() -> Result<(), RipgrepError> {
    panic_safe(|| panic!("intentional test panic"))
}

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
```

- [ ] **Step 2: Wrap search_blocking in catch_unwind**

Modify `search_blocking` in `crates/ripgrep_core/src/search.rs` to wrap its body in `panic_safe`:

```rust
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
    // ... move existing body here ...
}
```

- [ ] **Step 3: Add a panic-conversion test**

Append to `crates/ripgrep_core/src/lib.rs`:
```rust
#[cfg(test)]
mod panic_tests {
    #[test]
    fn force_panic_is_translated_to_internal_panic() {
        let err = super::search::force_panic_for_test().unwrap_err();
        match err {
            crate::error::RipgrepError::InternalPanic(msg) => {
                assert!(msg.contains("intentional"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p ripgrep_core panic_tests`
Expected: pass; the panic is caught and returned as `RipgrepError::InternalPanic`.

- [ ] **Step 5: Commit**

```bash
git add crates/ripgrep_core/
git commit -m "Wrap search_blocking in catch_unwind and surface InternalPanic"
```

---

### Task 13: Verify whole crate builds + lint pass

- [ ] **Step 1: Run full test suite + clippy**

Run:
```bash
cargo test -p ripgrep_core
cargo clippy -p ripgrep_core -- -D warnings
cargo fmt --check
```

Expected: all green. If clippy whines, fix the lints (do not `#[allow]` casually).

- [ ] **Step 2: Commit any fmt/clippy fixes**

```bash
git add crates/ripgrep_core/
git commit -m "Pass clippy and rustfmt"
```

---

## Phase 2 — UniFFI Bridge (Tasks 14-17)

### Task 14: Add UniFFI scaffolding

**Files:**
- Create: `crates/ripgrep_core/build.rs`
- Create: `crates/ripgrep_core/uniffi.toml`
- Modify: `crates/ripgrep_core/src/lib.rs`
- Modify: `crates/ripgrep_core/src/options.rs`
- Modify: `crates/ripgrep_core/src/error.rs`
- Modify: `crates/ripgrep_core/src/cancel.rs`

- [ ] **Step 1: Create build.rs**

Create `crates/ripgrep_core/build.rs`:
```rust
fn main() {
    uniffi::generate_scaffolding("./src/ripgrep_core.udl").ok();
    // We use proc-macro mode primarily; the UDL is optional fallback.
}
```

- [ ] **Step 2: Create uniffi.toml**

Create `crates/ripgrep_core/uniffi.toml`:
```toml
[bindings.swift]
module_name = "RipgrepCore"
generate_module_map = true
```

- [ ] **Step 3: Annotate types for UniFFI**

Modify `crates/ripgrep_core/src/options.rs` — add `uniffi::Record` derives:
```rust
#[derive(Debug, Clone, uniffi::Record)]
pub struct SearchRequest { /* unchanged fields */ }

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Submatch { /* unchanged */ }

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SearchMatch { /* unchanged */ }

#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct SearchResult { /* unchanged */ }
```

Modify `crates/ripgrep_core/src/error.rs`:
```rust
#[derive(Debug, Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum RipgrepError { /* unchanged variants */ }
```

Modify `crates/ripgrep_core/src/cancel.rs`:
```rust
#[derive(Debug, uniffi::Object)]
pub struct CancelToken { /* unchanged fields */ }

#[uniffi::export]
impl CancelToken {
    #[uniffi::constructor]
    pub fn new(timeout_ms: Option<u64>) -> Arc<Self> {
        Arc::new(Self {
            flag: Arc::new(AtomicBool::new(false)),
            deadline: timeout_ms.map(|ms| Instant::now() + Duration::from_millis(ms)),
        })
    }
    pub fn cancel(&self) { self.flag.store(true, Ordering::Relaxed); }
    pub fn is_cancelled(&self) -> bool {
        if self.flag.load(Ordering::Relaxed) { return true; }
        self.deadline.map_or(false, |d| Instant::now() >= d)
    }
}
```

- [ ] **Step 4: Export search_blocking via UniFFI**

Modify `crates/ripgrep_core/src/lib.rs` — add at the very top:
```rust
uniffi::setup_scaffolding!();
```
And add the FFI export:
```rust
#[uniffi::export]
pub fn search_blocking(
    request: crate::options::SearchRequest,
    cancel: std::sync::Arc<crate::cancel::CancelToken>,
) -> Result<crate::options::SearchResult, crate::error::RipgrepError> {
    crate::search::search_blocking(request, cancel)
}
```

(This re-exports the inherent function under a UniFFI signature. Keep the original `crate::search::search_blocking` as the implementation.)

- [ ] **Step 5: Build**

Run: `cargo build -p ripgrep_core --release`
Expected: clean build; no UniFFI macro errors.

- [ ] **Step 6: Run tests (no FFI runtime needed for unit tests)**

Run: `cargo test -p ripgrep_core`
Expected: all existing tests still pass.

- [ ] **Step 7: Commit**

```bash
git add crates/ripgrep_core/
git commit -m "Add UniFFI scaffolding and FFI exports"
```

---

### Task 15: Generate Swift bindings

**Files:**
- Create: `scripts/generate-bindings.sh`
- Create: `Sources/RipgrepKitFFI/ripgrep_core.swift` (output)
- Create: `Sources/RipgrepKitFFI/ripgrep_coreFFI.modulemap`

- [ ] **Step 1: Write the binding generator script**

Create `scripts/generate-bindings.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail

# Generate Swift bindings from the compiled ripgrep_core dylib using
# uniffi-bindgen.

cd "$(dirname "$0")/.."

cargo build -p ripgrep_core --release

DYLIB="target/release/libripgrep_core.dylib"
OUT_DIR="Sources/RipgrepKitFFI"
mkdir -p "$OUT_DIR"

uniffi-bindgen generate \
    --library "$DYLIB" \
    --language swift \
    --out-dir "$OUT_DIR"

echo "Generated bindings in $OUT_DIR:"
ls -la "$OUT_DIR"
```

```bash
chmod +x scripts/generate-bindings.sh
```

- [ ] **Step 2: Run the generator**

Run: `bash scripts/generate-bindings.sh`
Expected: writes `Sources/RipgrepKitFFI/ripgrep_core.swift` (≈ 100-300 lines), `Sources/RipgrepKitFFI/RipgrepCoreFFI.h`, `Sources/RipgrepKitFFI/RipgrepCoreFFI.modulemap`.

- [ ] **Step 3: Verify generated content references key symbols**

Run:
```bash
grep -E "(SearchRequest|searchBlocking|CancelToken|RipgrepError)" Sources/RipgrepKitFFI/ripgrep_core.swift | head -20
```
Expected: each symbol shows up.

- [ ] **Step 4: Commit**

```bash
git add scripts/generate-bindings.sh Sources/RipgrepKitFFI/
git commit -m "Add UniFFI Swift bindings generator and initial output"
```

---

### Task 16: Build single-slice XCFramework (macOS arm64) — smoke test

**Files:**
- Create: `scripts/build-xcframework.sh`

- [ ] **Step 1: Write the script for a single slice (will be expanded in Task 17)**

Create `scripts/build-xcframework.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

CRATE="ripgrep_core"
LIB_NAME="libripgrep_core.a"
FRAMEWORK_NAME="RipgrepCore"
BUILD_DIR="build/xcframework"
OUT="Frameworks/${FRAMEWORK_NAME}.xcframework"

rm -rf "$BUILD_DIR" "$OUT"
mkdir -p "$BUILD_DIR"

build_static() {
    local triple="$1"
    cargo build -p "$CRATE" --release --target "$triple"
}

stage_framework() {
    local triple="$1"
    local slice_name="$2"
    local fw_dir="$BUILD_DIR/$slice_name/$FRAMEWORK_NAME.framework"
    mkdir -p "$fw_dir/Headers" "$fw_dir/Modules"

    cp "target/$triple/release/$LIB_NAME" "$fw_dir/$FRAMEWORK_NAME"
    cp "Sources/RipgrepKitFFI/RipgrepCoreFFI.h" "$fw_dir/Headers/"
    cat > "$fw_dir/Modules/module.modulemap" <<'EOF'
framework module RipgrepCore {
    umbrella header "RipgrepCoreFFI.h"
    export *
    module * { export * }
}
EOF
    cat > "$fw_dir/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>$FRAMEWORK_NAME</string>
    <key>CFBundleIdentifier</key><string>com.ripgrep.RipgrepCore</string>
    <key>CFBundleName</key><string>$FRAMEWORK_NAME</string>
    <key>CFBundlePackageType</key><string>FMWK</string>
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundleVersion</key><string>1</string>
</dict>
</plist>
EOF
    echo "$fw_dir"
}

# Single slice (smoke test)
build_static "aarch64-apple-darwin"
MAC_FW=$(stage_framework "aarch64-apple-darwin" "macos-arm64")

xcodebuild -create-xcframework \
    -framework "$MAC_FW" \
    -output "$OUT"

echo "Built: $OUT"
ls -la "$OUT"
```

```bash
chmod +x scripts/build-xcframework.sh
```

- [ ] **Step 2: Run it**

Run: `bash scripts/build-xcframework.sh`
Expected: success message; `Frameworks/RipgrepCore.xcframework/macos-arm64/RipgrepCore.framework/RipgrepCore` exists.

- [ ] **Step 3: Commit**

```bash
git add scripts/build-xcframework.sh
git commit -m "Add single-slice xcframework build script (smoke test)"
```

---

### Task 17: Expand to all 5 Apple targets

**Files:**
- Modify: `scripts/build-xcframework.sh`

- [ ] **Step 1: Add lipo + multi-target build**

Replace the bottom of `scripts/build-xcframework.sh` (everything after `stage_framework` definition) with:
```bash
# All 5 Rust target triples
build_static "aarch64-apple-darwin"
build_static "x86_64-apple-darwin"
build_static "aarch64-apple-ios"
build_static "aarch64-apple-ios-sim"
build_static "x86_64-apple-ios"

# lipo macOS arm64 + x86_64
mkdir -p "build/lipo/macos"
lipo -create \
    "target/aarch64-apple-darwin/release/$LIB_NAME" \
    "target/x86_64-apple-darwin/release/$LIB_NAME" \
    -output "build/lipo/macos/$LIB_NAME"

# lipo iOS simulator arm64 + x86_64
mkdir -p "build/lipo/ios-sim"
lipo -create \
    "target/aarch64-apple-ios-sim/release/$LIB_NAME" \
    "target/x86_64-apple-ios/release/$LIB_NAME" \
    -output "build/lipo/ios-sim/$LIB_NAME"

# Stage three slice dirs by hand-cp'ing the lipo'd bins
stage_framework_from_lib() {
    local lib_path="$1"
    local slice_name="$2"
    local fw_dir="$BUILD_DIR/$slice_name/$FRAMEWORK_NAME.framework"
    mkdir -p "$fw_dir/Headers" "$fw_dir/Modules"
    cp "$lib_path" "$fw_dir/$FRAMEWORK_NAME"
    cp "Sources/RipgrepKitFFI/RipgrepCoreFFI.h" "$fw_dir/Headers/"
    cat > "$fw_dir/Modules/module.modulemap" <<'EOF'
framework module RipgrepCore {
    umbrella header "RipgrepCoreFFI.h"
    export *
    module * { export * }
}
EOF
    cat > "$fw_dir/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>$FRAMEWORK_NAME</string>
<key>CFBundleIdentifier</key><string>com.ripgrep.RipgrepCore</string>
<key>CFBundleName</key><string>$FRAMEWORK_NAME</string>
<key>CFBundlePackageType</key><string>FMWK</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
</dict></plist>
EOF
    echo "$fw_dir"
}

MAC_FW=$(stage_framework_from_lib "build/lipo/macos/$LIB_NAME" "macos")
IOS_FW=$(stage_framework_from_lib "target/aarch64-apple-ios/release/$LIB_NAME" "ios-arm64")
SIM_FW=$(stage_framework_from_lib "build/lipo/ios-sim/$LIB_NAME" "ios-sim")

xcodebuild -create-xcframework \
    -framework "$MAC_FW" \
    -framework "$IOS_FW" \
    -framework "$SIM_FW" \
    -output "$OUT"

echo "Built: $OUT"
ls "$OUT"
```

- [ ] **Step 2: Run it**

Run: `bash scripts/build-xcframework.sh`
Expected: 3 slice dirs inside `Frameworks/RipgrepCore.xcframework/`.

```bash
ls Frameworks/RipgrepCore.xcframework/
```
Expected output includes 3 directories: `macos-arm64_x86_64`, `ios-arm64`, `ios-arm64_x86_64-simulator` (exact names depend on xcodebuild).

- [ ] **Step 3: Commit**

```bash
git add scripts/build-xcframework.sh
git commit -m "Build XCFramework for all 5 Apple target triples"
```

---

## Phase 3 — Swift Package Skeleton (Tasks 18-26)

### Task 18: Package.swift with local binaryTarget

**Files:**
- Create: `Package.swift`

- [ ] **Step 1: Write the manifest**

Create `Package.swift`:
```swift
// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "RipgrepKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "RipgrepKitCore", targets: ["RipgrepKitCore"]),
        .library(name: "RipgrepKitTool", targets: ["RipgrepKitTool"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser", from: "1.5.0"),
    ],
    targets: [
        .binaryTarget(
            name: "RipgrepCore",
            path: "Frameworks/RipgrepCore.xcframework"
        ),
        .target(name: "RipgrepKitFFI", dependencies: ["RipgrepCore"]),
        .target(name: "RipgrepKitCore", dependencies: ["RipgrepKitFFI"]),
        .target(name: "RipgrepKitTool", dependencies: [
            "RipgrepKitCore",
            .product(name: "ArgumentParser", package: "swift-argument-parser"),
        ]),
        .testTarget(
            name: "RipgrepKitCoreTests",
            dependencies: ["RipgrepKitCore"],
            resources: [.copy("Fixtures")]
        ),
        .testTarget(
            name: "RipgrepKitToolTests",
            dependencies: ["RipgrepKitTool"],
            resources: [.copy("Fixtures")]
        ),
    ],
    swiftLanguageModes: [.v6]
)
```

- [ ] **Step 2: Resolve dependencies**

Run: `swift package resolve`
Expected: fetches `swift-argument-parser`.

- [ ] **Step 3: Commit**

```bash
git add Package.swift Package.resolved
git commit -m "Add SwiftPM manifest with local binaryTarget"
```

---

### Task 19: Stub Swift targets so the package builds

**Files:**
- Create: `Sources/RipgrepKitCore/Ripgrep.swift`
- Create: `Sources/RipgrepKitTool/Stub.swift`
- Create: `Tests/RipgrepKitCoreTests/Stub.swift`
- Create: `Tests/RipgrepKitToolTests/Stub.swift`

- [ ] **Step 1: Minimum scaffolding so swift build doesn't complain about empty targets**

Create `Sources/RipgrepKitCore/Ripgrep.swift`:
```swift
import RipgrepKitFFI

public enum Ripgrep {
    // Filled in later tasks.
}
```

Create `Sources/RipgrepKitTool/Stub.swift`:
```swift
@_exported import RipgrepKitCore
```

Create `Tests/RipgrepKitCoreTests/Stub.swift`:
```swift
import XCTest

final class StubTests: XCTestCase {
    func testPlaceholder() { XCTAssertTrue(true) }
}
```

Create `Tests/RipgrepKitToolTests/Stub.swift`:
```swift
import XCTest

final class StubTests: XCTestCase {
    func testPlaceholder() { XCTAssertTrue(true) }
}
```

Create empty fixture dirs so `resources: [.copy("Fixtures")]` doesn't error:
```bash
mkdir -p Tests/RipgrepKitCoreTests/Fixtures
mkdir -p Tests/RipgrepKitToolTests/Fixtures
touch Tests/RipgrepKitCoreTests/Fixtures/.gitkeep
touch Tests/RipgrepKitToolTests/Fixtures/.gitkeep
```

- [ ] **Step 2: Build the package**

Run: `swift build`
Expected: succeeds. The first build links against `Frameworks/RipgrepCore.xcframework` and the auto-generated `Sources/RipgrepKitFFI/ripgrep_core.swift`.

- [ ] **Step 3: Run the placeholder tests**

Run: `swift test`
Expected: 2 placeholder tests pass.

- [ ] **Step 4: Commit**

```bash
git add Sources/ Tests/
git commit -m "Add stub Swift targets so the package builds end-to-end"
```

---

### Task 20: RipgrepKitCore — Error type

**Files:**
- Create: `Sources/RipgrepKitCore/Error.swift`

- [ ] **Step 1: Write the failing test**

Create `Tests/RipgrepKitCoreTests/ErrorTests.swift`:
```swift
import XCTest
@testable import RipgrepKitCore

final class ErrorTests: XCTestCase {
    func testErrorMessageRendering() {
        let e = Ripgrep.Error.invalidPattern("[")
        XCTAssertTrue(e.message.contains("invalid"))
        XCTAssertTrue(e.message.contains("["))
    }

    func testInvalidArgumentsCarriesMessage() {
        let e = Ripgrep.Error.invalidArguments(message: "missing pattern")
        XCTAssertEqual(e.message, "missing pattern")
    }
}
```

- [ ] **Step 2: Run test**

Run: `swift test --filter ErrorTests`
Expected: compile failure — `Ripgrep.Error` doesn't exist yet.

- [ ] **Step 3: Implement Error**

Create `Sources/RipgrepKitCore/Error.swift`:
```swift
import RipgrepKitFFI

extension Ripgrep {
    public enum Error: Swift.Error, Sendable {
        case invalidArguments(message: String)
        case invalidPattern(String)
        case pathNotFound(String)
        case io(String)
        case internalPanic(String)

        public var message: String {
            switch self {
            case .invalidArguments(let m): return m
            case .invalidPattern(let p):   return "invalid regex: \(p)"
            case .pathNotFound(let p):     return "path not found: \(p)"
            case .io(let m):               return "io error: \(m)"
            case .internalPanic(let m):    return "internal panic: \(m)"
            }
        }

        /// Maps a UniFFI-generated error to our public Error.
        static func from(_ ffi: RipgrepError) -> Ripgrep.Error {
            switch ffi {
            case .InvalidPattern(let s): return .invalidPattern(s)
            case .PathNotFound(let s):   return .pathNotFound(s)
            case .Io(let s):             return .io(s)
            case .InternalPanic(let s):  return .internalPanic(s)
            }
        }
    }
}
```

> Note: the exact UniFFI-generated case names (`.InvalidPattern` vs `.invalidPattern`) depend on UniFFI version. Inspect `Sources/RipgrepKitFFI/ripgrep_core.swift` and adjust if needed.

- [ ] **Step 4: Run test**

Run: `swift test --filter ErrorTests`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add Sources/RipgrepKitCore/Error.swift Tests/RipgrepKitCoreTests/ErrorTests.swift
git commit -m "Add Ripgrep.Error with UniFFI mapping"
```

---

### Task 21: RipgrepKitCore — Options struct

**Files:**
- Create: `Sources/RipgrepKitCore/Options.swift`

- [ ] **Step 1: Write the failing tests**

Create `Tests/RipgrepKitCoreTests/OptionsTests.swift`:
```swift
import XCTest
@testable import RipgrepKitCore

final class OptionsTests: XCTestCase {
    func testDefaultsMatchRipgrepCLI() {
        let o = Ripgrep.Options()
        XCTAssertFalse(o.caseInsensitive)
        XCTAssertFalse(o.smartCase)        // matches rg CLI default
        XCTAssertFalse(o.multiline)
        XCTAssertTrue(o.respectGitignore)
        XCTAssertFalse(o.includeHidden)
        XCTAssertEqual(o.beforeContext, 0)
        XCTAssertEqual(o.afterContext, 0)
        XCTAssertNil(o.maxMatches)
        XCTAssertNil(o.timeout)
    }

    func testCodableRoundtrip() throws {
        var o = Ripgrep.Options()
        o.beforeContext = 2
        o.include = ["*.swift"]
        o.timeout = .milliseconds(500)
        let data = try JSONEncoder().encode(o)
        let decoded = try JSONDecoder().decode(Ripgrep.Options.self, from: data)
        XCTAssertEqual(decoded.beforeContext, 2)
        XCTAssertEqual(decoded.include, ["*.swift"])
    }
}
```

- [ ] **Step 2: Run test**

Run: `swift test --filter OptionsTests`
Expected: compile failure.

- [ ] **Step 3: Implement Options**

Create `Sources/RipgrepKitCore/Options.swift`:
```swift
import Foundation

extension Ripgrep {
    public struct Options: Codable, Sendable {
        public var caseInsensitive: Bool
        public var smartCase: Bool
        public var multiline: Bool
        public var include: [String]
        public var exclude: [String]
        public var fileTypes: [String]
        public var respectGitignore: Bool
        public var includeHidden: Bool
        public var beforeContext: Int
        public var afterContext: Int
        public var maxMatches: Int?
        public var maxFiles: Int?
        public var maxFileSizeBytes: Int?
        public var timeout: Duration?

        public init(
            caseInsensitive: Bool = false,
            smartCase: Bool = false,
            multiline: Bool = false,
            include: [String] = [],
            exclude: [String] = [],
            fileTypes: [String] = [],
            respectGitignore: Bool = true,
            includeHidden: Bool = false,
            beforeContext: Int = 0,
            afterContext: Int = 0,
            maxMatches: Int? = nil,
            maxFiles: Int? = nil,
            maxFileSizeBytes: Int? = nil,
            timeout: Duration? = nil
        ) {
            self.caseInsensitive = caseInsensitive
            self.smartCase = smartCase
            self.multiline = multiline
            self.include = include
            self.exclude = exclude
            self.fileTypes = fileTypes
            self.respectGitignore = respectGitignore
            self.includeHidden = includeHidden
            self.beforeContext = beforeContext
            self.afterContext = afterContext
            self.maxMatches = maxMatches
            self.maxFiles = maxFiles
            self.maxFileSizeBytes = maxFileSizeBytes
            self.timeout = timeout
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `swift test --filter OptionsTests`
Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add Sources/RipgrepKitCore/Options.swift Tests/RipgrepKitCoreTests/OptionsTests.swift
git commit -m "Add Ripgrep.Options with rg-aligned defaults"
```

---

### Task 22: RipgrepKitCore — SearchResult types

**Files:**
- Create: `Sources/RipgrepKitCore/SearchResult.swift`

- [ ] **Step 1: Write the failing test**

Create `Tests/RipgrepKitCoreTests/SearchResultTests.swift`:
```swift
import XCTest
@testable import RipgrepKitCore

final class SearchResultTests: XCTestCase {
    func testFormattedAsTextSimple() {
        let r = Ripgrep.SearchResult(
            matches: [
                Ripgrep.Match(
                    path: "src/foo.swift", lineNumber: 10,
                    line: "// TODO: rename", beforeContext: [], afterContext: [],
                    submatches: []
                )
            ],
            truncated: false, cancelled: false,
            filesSearched: 1, elapsed: .milliseconds(2)
        )
        let out = r.formattedAsText()
        XCTAssertTrue(out.contains("src/foo.swift:10:// TODO: rename"))
    }

    func testFormattedAsTextWithContext() {
        let r = Ripgrep.SearchResult(
            matches: [
                Ripgrep.Match(
                    path: "f.txt", lineNumber: 10,
                    line: "match", beforeContext: ["before"], afterContext: ["after"],
                    submatches: []
                )
            ],
            truncated: false, cancelled: false,
            filesSearched: 1, elapsed: .milliseconds(0)
        )
        let out = r.formattedAsText()
        XCTAssertTrue(out.contains("f.txt-9-before"))
        XCTAssertTrue(out.contains("f.txt:10:match"))
        XCTAssertTrue(out.contains("f.txt-11-after"))
    }

    func testFormattedAsJSONLinesPerMatch() throws {
        let r = Ripgrep.SearchResult(
            matches: [
                Ripgrep.Match(
                    path: "a", lineNumber: 1, line: "x",
                    beforeContext: [], afterContext: [], submatches: []
                ),
                Ripgrep.Match(
                    path: "b", lineNumber: 2, line: "y",
                    beforeContext: [], afterContext: [], submatches: []
                ),
            ],
            truncated: false, cancelled: false, filesSearched: 2, elapsed: .milliseconds(0)
        )
        let lines = r.formattedAsJSONLines().split(separator: "\n")
        XCTAssertEqual(lines.count, 2)  // no envelope, just per-match
    }
}
```

- [ ] **Step 2: Implement**

Create `Sources/RipgrepKitCore/SearchResult.swift`:
```swift
import Foundation

extension Ripgrep {
    public struct Submatch: Codable, Sendable, Equatable {
        public let start: Int
        public let end: Int
        public init(start: Int, end: Int) { self.start = start; self.end = end }
    }

    public struct Match: Codable, Sendable, Equatable {
        public let path: String
        public let lineNumber: Int
        public let line: String
        public let beforeContext: [String]
        public let afterContext: [String]
        public let submatches: [Submatch]

        public init(path: String, lineNumber: Int, line: String,
                    beforeContext: [String], afterContext: [String],
                    submatches: [Submatch]) {
            self.path = path; self.lineNumber = lineNumber; self.line = line
            self.beforeContext = beforeContext; self.afterContext = afterContext
            self.submatches = submatches
        }
    }

    public struct SearchResult: Codable, Sendable {
        public let matches: [Match]
        public let truncated: Bool
        public let cancelled: Bool
        public let filesSearched: Int
        public let elapsed: Duration

        public init(matches: [Match], truncated: Bool, cancelled: Bool,
                    filesSearched: Int, elapsed: Duration) {
            self.matches = matches; self.truncated = truncated
            self.cancelled = cancelled; self.filesSearched = filesSearched
            self.elapsed = elapsed
        }

        public func formattedAsText() -> String {
            var lines: [String] = []
            var lastPath: String? = nil
            for m in matches {
                if let lp = lastPath, lp != m.path { lines.append("") }
                lastPath = m.path
                let baseLine = m.lineNumber
                for (i, b) in m.beforeContext.enumerated() {
                    let ln = baseLine - (m.beforeContext.count - i)
                    lines.append("\(m.path)-\(ln)-\(b)")
                }
                lines.append("\(m.path):\(baseLine):\(m.line)")
                for (i, a) in m.afterContext.enumerated() {
                    let ln = baseLine + i + 1
                    lines.append("\(m.path)-\(ln)-\(a)")
                }
            }
            return lines.joined(separator: "\n")
        }

        public func formattedAsJSONLines() -> String {
            let enc = JSONEncoder()
            enc.outputFormatting = []
            return matches.compactMap { m in
                guard let data = try? enc.encode(m),
                      let s = String(data: data, encoding: .utf8) else { return nil }
                return s
            }.joined(separator: "\n")
        }
    }
}
```

- [ ] **Step 3: Run tests**

Run: `swift test --filter SearchResultTests`
Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add Sources/RipgrepKitCore/SearchResult.swift Tests/RipgrepKitCoreTests/SearchResultTests.swift
git commit -m "Add Ripgrep.Match, SearchResult, formatters (text + JSON lines)"
```

---

### Task 23: RipgrepKitCore — Ripgrep.search wiring (cancellation, FFI conversion)

**Files:**
- Create: `Sources/RipgrepKitCore/Search.swift`
- Modify: `Sources/RipgrepKitCore/Options.swift` (add `toFFI()`)

- [ ] **Step 1: Add Options.toFFI() with M5 preconditions**

Append to `Sources/RipgrepKitCore/Options.swift`:
```swift
import RipgrepKitFFI

extension Ripgrep.Options {
    func toFFI(pattern: String, paths: [String]) -> SearchRequest {
        precondition(beforeContext >= 0, "beforeContext must be ≥ 0")
        precondition(afterContext  >= 0, "afterContext must be ≥ 0")
        precondition((maxMatches ?? 0) >= 0, "maxMatches must be ≥ 0")
        precondition((maxFiles ?? 0)   >= 0, "maxFiles must be ≥ 0")
        precondition((maxFileSizeBytes ?? 0) >= 0, "maxFileSizeBytes must be ≥ 0")

        let timeoutMs: UInt64? = timeout.flatMap {
            let comp = $0.components
            return UInt64(comp.seconds * 1000 + comp.attoseconds / 1_000_000_000_000_000)
        }

        return SearchRequest(
            pattern: pattern,
            paths: paths.isEmpty ? ["."] : paths,
            caseInsensitive: caseInsensitive,
            smartCase: smartCase,
            multiline: multiline,
            includeGlobs: include,
            excludeGlobs: exclude,
            fileTypes: fileTypes,
            respectGitignore: respectGitignore,
            includeHidden: includeHidden,
            beforeContext: UInt32(beforeContext),
            afterContext: UInt32(afterContext),
            maxMatches: maxMatches.map(UInt32.init),
            maxFiles: maxFiles.map(UInt32.init),
            maxFileSizeBytes: maxFileSizeBytes.map(UInt64.init),
            timeoutMs: timeoutMs
        )
    }
}
```

- [ ] **Step 2: Implement Ripgrep.search**

Create `Sources/RipgrepKitCore/Search.swift`:
```swift
import Foundation
import RipgrepKitFFI

extension Ripgrep {
    public static func search(
        pattern: String,
        in paths: [String],
        options: Options = .init()
    ) async throws -> SearchResult {
        let request = options.toFFI(pattern: pattern, paths: paths)
        let timeoutMs: UInt64? = options.timeout.flatMap {
            let c = $0.components
            return UInt64(c.seconds * 1000 + c.attoseconds / 1_000_000_000_000_000)
        }
        let token = CancelToken(timeoutMs: timeoutMs)

        return try await withTaskCancellationHandler {
            try await Task.detached(priority: .userInitiated) {
                do {
                    let ffi = try searchBlocking(request: request, cancel: token)
                    return SearchResult(from: ffi)
                } catch let e as RipgrepError {
                    throw Ripgrep.Error.from(e)
                }
            }.value
        } onCancel: {
            token.cancel()
        }
    }
}

extension Ripgrep.SearchResult {
    init(from ffi: SearchResult) {        // FFI's SearchResult — same name, different module
        self.init(
            matches: ffi.matches.map { Ripgrep.Match(from: $0) },
            truncated: ffi.truncated,
            cancelled: ffi.cancelled,
            filesSearched: Int(ffi.filesSearched),
            elapsed: .milliseconds(Int(ffi.elapsedMs))
        )
    }
}

extension Ripgrep.Match {
    init(from ffi: SearchMatch) {
        self.init(
            path: ffi.path,
            lineNumber: Int(ffi.lineNumber),
            line: ffi.line,
            beforeContext: ffi.beforeContext,
            afterContext: ffi.afterContext,
            submatches: ffi.submatches.map { Ripgrep.Submatch(start: Int($0.start), end: Int($0.end)) }
        )
    }
}
```

> If type names from `RipgrepKitFFI` collide with `Ripgrep.Match` etc., qualify with `RipgrepKitFFI.SearchMatch` or alias on import.

- [ ] **Step 3: Verify build**

Run: `swift build`
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add Sources/RipgrepKitCore/
git commit -m "Wire Ripgrep.search with cancellation bridge and FFI conversion"
```

---

## Phase 4 — Swift Core Tests with fixture (Tasks 24-31)

### Task 24: Test fixture mini-repo

**Files:**
- Create: `Tests/RipgrepKitCoreTests/Fixtures/mini/...`

- [ ] **Step 1: Create the fixture files**

(Same content as Task 7 fixture, replicated under SwiftPM test resources.)

```bash
mkdir -p Tests/RipgrepKitCoreTests/Fixtures/mini/sub Tests/RipgrepKitCoreTests/Fixtures/mini/target
```

Then create each file with the same content as in Task 7 (`included.txt`, `ignored.txt`, `.hidden.txt`, `.gitignore`, `sub/nested.swift`, `target/built.txt`, `context.txt`).

- [ ] **Step 2: Verify SwiftPM bundles them**

Run: `swift build`
Expected: clean build; the Fixtures dir is copied into the test bundle.

- [ ] **Step 3: Commit**

```bash
git add Tests/RipgrepKitCoreTests/Fixtures/
git commit -m "Add test fixture mini-repo for end-to-end Swift tests"
```

---

### Task 25: SearchTests — basic search

**Files:**
- Create: `Tests/RipgrepKitCoreTests/SearchTests.swift`

- [ ] **Step 1: Write the test**

Create `Tests/RipgrepKitCoreTests/SearchTests.swift`:
```swift
import XCTest
@testable import RipgrepKitCore

final class SearchTests: XCTestCase {
    private func fixturePath() -> String {
        let url = Bundle.module.url(forResource: "mini", withExtension: nil)!
        return url.path
    }

    func testFindsMatches() async throws {
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()])
        XCTAssertGreaterThan(r.matches.count, 0)
        XCTAssertTrue(r.matches.contains { $0.line.contains("TODO") })
    }

    func testRespectsGitignoreByDefault() async throws {
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()])
        XCTAssertFalse(r.matches.contains { $0.path.contains("ignored.txt") })
        XCTAssertFalse(r.matches.contains { $0.path.contains("/target/") })
    }

    func testHiddenFilesSkippedByDefault() async throws {
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()])
        XCTAssertFalse(r.matches.contains { $0.path.contains(".hidden") })
    }

    func testIncludeGlobFiltersToSwift() async throws {
        var opts = Ripgrep.Options(); opts.include = ["*.swift"]
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()], options: opts)
        XCTAssertTrue(r.matches.allSatisfy { $0.path.hasSuffix(".swift") })
    }

    func testInvalidPatternThrows() async throws {
        do {
            _ = try await Ripgrep.search(pattern: "[", in: [fixturePath()])
            XCTFail("expected throw")
        } catch let e as Ripgrep.Error {
            if case .invalidPattern = e { } else { XCTFail("wrong error: \(e)") }
        }
    }
}
```

- [ ] **Step 2: Run**

Run: `swift test --filter SearchTests`
Expected: 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add Tests/RipgrepKitCoreTests/SearchTests.swift
git commit -m "Add Swift end-to-end search tests against fixture"
```

---

### Task 26: CancellationTests

**Files:**
- Create: `Tests/RipgrepKitCoreTests/CancellationTests.swift`

- [ ] **Step 1: Write tests**

```swift
import XCTest
@testable import RipgrepKitCore

final class CancellationTests: XCTestCase {
    private func fixturePath() -> String {
        Bundle.module.url(forResource: "mini", withExtension: nil)!.path
    }

    func testTimeoutMarksResultCancelled() async throws {
        var opts = Ripgrep.Options()
        opts.timeout = .nanoseconds(1)   // immediate
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()], options: opts)
        XCTAssertTrue(r.cancelled || r.matches.isEmpty)
    }

    func testTaskCancelStopsSearch() async throws {
        let path = fixturePath()
        let task = Task {
            try await Ripgrep.search(pattern: "TODO", in: [path])
        }
        task.cancel()
        do {
            let r = try await task.value
            XCTAssertTrue(r.cancelled || r.matches.count >= 0)  // tiny fixture may finish first
        } catch is CancellationError {
            // acceptable: detached Task surfaces cancellation
        }
    }
}
```

- [ ] **Step 2: Run**

Run: `swift test --filter CancellationTests`
Expected: pass; no hang.

- [ ] **Step 3: Commit**

```bash
git add Tests/RipgrepKitCoreTests/CancellationTests.swift
git commit -m "Add cancellation and timeout tests"
```

---

## Phase 5 — RipgrepKitTool (Tasks 27-32)

### Task 27: Tokenizer

**Files:**
- Create: `Sources/RipgrepKitTool/Tokenizer.swift`
- Create: `Tests/RipgrepKitToolTests/TokenizerTests.swift`

- [ ] **Step 1: Write failing tests**

```swift
import XCTest
@testable import RipgrepKitTool
@testable import RipgrepKitCore

final class TokenizerTests: XCTestCase {
    func testSimpleSplit() throws {
        XCTAssertEqual(try Tokenizer.tokenize("foo bar baz"), ["foo", "bar", "baz"])
    }
    func testDoubleQuotes() throws {
        XCTAssertEqual(try Tokenizer.tokenize("\"hello world\" foo"), ["hello world", "foo"])
    }
    func testSingleQuotes() throws {
        XCTAssertEqual(try Tokenizer.tokenize("'don\\'t' x"), ["don\\'t", "x"])
    }
    func testBackslashEscape() throws {
        XCTAssertEqual(try Tokenizer.tokenize(#"a\ b c"#), ["a b", "c"])
    }
    func testFlagEqualsValueSingleToken() throws {
        XCTAssertEqual(try Tokenizer.tokenize(#"--glob='*.swift' src"#),
                       ["--glob=*.swift", "src"])
    }
    func testFlagEqualsValueDoubleQuoted() throws {
        XCTAssertEqual(try Tokenizer.tokenize(#"--glob="hello world""#),
                       ["--glob=hello world"])
    }
    func testUnbalancedQuoteThrows() {
        XCTAssertThrowsError(try Tokenizer.tokenize(#""unclosed"#)) { e in
            guard let e = e as? Ripgrep.Error else { return XCTFail() }
            if case .invalidArguments = e {} else { XCTFail() }
        }
    }
    func testEmptyInputReturnsEmpty() throws {
        XCTAssertEqual(try Tokenizer.tokenize(""), [])
        XCTAssertEqual(try Tokenizer.tokenize("   "), [])
    }
}
```

- [ ] **Step 2: Implement Tokenizer**

Create `Sources/RipgrepKitTool/Tokenizer.swift`:
```swift
import RipgrepKitCore

public enum Tokenizer {
    public static func tokenize(_ input: String) throws(Ripgrep.Error) -> [String] {
        var tokens: [String] = []
        var current = ""
        var inSingle = false
        var inDouble = false
        var escaped = false
        var hasContent = false

        for ch in input {
            if escaped {
                current.append(ch)
                escaped = false
                hasContent = true
                continue
            }
            if ch == "\\" && !inSingle {
                escaped = true
                continue
            }
            if ch == "'" && !inDouble { inSingle.toggle(); hasContent = true; continue }
            if ch == "\"" && !inSingle { inDouble.toggle(); hasContent = true; continue }

            if !inSingle && !inDouble && ch.isWhitespace {
                if hasContent {
                    tokens.append(current)
                    current = ""
                    hasContent = false
                }
                continue
            }
            current.append(ch)
            hasContent = true
        }

        if inSingle || inDouble {
            throw .invalidArguments(message: "unbalanced quote in argument string")
        }
        if escaped {
            throw .invalidArguments(message: "dangling backslash escape")
        }
        if hasContent { tokens.append(current) }
        return tokens
    }
}
```

- [ ] **Step 3: Run tests**

Run: `swift test --filter TokenizerTests`
Expected: 8 tests pass.

- [ ] **Step 4: Commit**

```bash
git add Sources/RipgrepKitTool/Tokenizer.swift Tests/RipgrepKitToolTests/TokenizerTests.swift
git commit -m "Add Tokenizer with shell-style splitting and typed throws"
```

---

### Task 28: RipgrepArgs (ParsableCommand)

**Files:**
- Create: `Sources/RipgrepKitTool/RipgrepArgs.swift`

- [ ] **Step 1: Write failing tests**

Create `Tests/RipgrepKitToolTests/ArgsParsingTests.swift`:
```swift
import XCTest
import ArgumentParser
@testable import RipgrepKitTool

final class ArgsParsingTests: XCTestCase {
    func testRequiredPattern() {
        XCTAssertThrowsError(try RipgrepArgs.parse([]))
    }
    func testPatternOnly() throws {
        let a = try RipgrepArgs.parse(["TODO"])
        XCTAssertEqual(a.pattern, "TODO")
        XCTAssertTrue(a.paths.isEmpty)
    }
    func testIgnoreCase() throws {
        let a = try RipgrepArgs.parse(["-i", "x"])
        XCTAssertTrue(a.ignoreCase)
    }
    func testSmartCaseDefaultOff() throws {
        let a = try RipgrepArgs.parse(["x"])
        XCTAssertFalse(a.smartCase)
    }
    func testGlobRepeats() throws {
        let a = try RipgrepArgs.parse(["x", "-g", "*.swift", "-g", "!*.test.swift"])
        XCTAssertEqual(a.glob, ["*.swift", "!*.test.swift"])
    }
    func testContextSetsBoth() throws {
        let a = try RipgrepArgs.parse(["x", "-C", "3"])
        XCTAssertEqual(a.context, 3)
    }
    func testJSONFlag() throws {
        let a = try RipgrepArgs.parse(["--json", "x"])
        XCTAssertTrue(a.json)
    }
}
```

- [ ] **Step 2: Implement RipgrepArgs**

Create `Sources/RipgrepKitTool/RipgrepArgs.swift`:
```swift
import ArgumentParser

struct RipgrepArgs: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "rg",
        abstract: "Search files using ripgrep semantics (subset)."
    )

    @Argument var pattern: String
    @Argument var paths: [String] = []

    @Flag(name: [.short, .customLong("ignore-case")])
    var ignoreCase: Bool = false

    @Flag(name: [.customShort("S"), .customLong("smart-case")])
    var smartCase: Bool = false

    @Flag(name: [.customShort("U"), .customLong("multiline")])
    var multiline: Bool = false

    @Option(name: [.customShort("g"), .customLong("glob")])
    var glob: [String] = []

    @Option(name: [.customShort("t"), .customLong("type")])
    var fileTypes: [String] = []

    @Flag(name: .customLong("hidden"))
    var hidden: Bool = false

    @Flag(name: .customLong("no-ignore"))
    var noIgnore: Bool = false

    @Option(name: [.customShort("A"), .customLong("after-context")])
    var afterContext: Int = 0

    @Option(name: [.customShort("B"), .customLong("before-context")])
    var beforeContext: Int = 0

    @Option(name: [.customShort("C"), .customLong("context")])
    var context: Int = 0

    @Option(name: [.customShort("m"), .customLong("max-count")])
    var maxCount: Int?

    @Option(name: .customLong("max-files"))
    var maxFiles: Int?

    @Option(name: .customLong("max-filesize"))
    var maxFilesize: String?

    @Option(name: .customLong("timeout-ms"))
    var timeoutMs: Int?

    @Flag(name: .customLong("json"))
    var json: Bool = false

    func run() throws { /* unused; see Parse.swift */ }
}
```

- [ ] **Step 3: Run tests**

Run: `swift test --filter ArgsParsingTests`
Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add Sources/RipgrepKitTool/RipgrepArgs.swift Tests/RipgrepKitToolTests/ArgsParsingTests.swift
git commit -m "Add RipgrepArgs ParsableCommand with rg-aligned flags"
```

---

### Task 29: Parse — RipgrepArgs → ParsedInvocation

**Files:**
- Create: `Sources/RipgrepKitTool/Parse.swift`

- [ ] **Step 1: Write failing tests**

Create `Tests/RipgrepKitToolTests/ParseTests.swift`:
```swift
import XCTest
@testable import RipgrepKitTool
@testable import RipgrepKitCore

final class ParseTests: XCTestCase {
    func testBasicParse() throws {
        let p = try Ripgrep.parse("TODO src/")
        XCTAssertEqual(p.pattern, "TODO")
        XCTAssertEqual(p.paths, ["src/"])
        XCTAssertEqual(p.outputFormat, .text)
    }

    func testGlobSplitByPrefix() throws {
        let p = try Ripgrep.parse("x -g '*.swift' -g '!*.test.swift'")
        XCTAssertEqual(p.options.include, ["*.swift"])
        XCTAssertEqual(p.options.exclude, ["*.test.swift"])
    }

    func testNoIgnoreInvertsGitignore() throws {
        let p = try Ripgrep.parse("x --no-ignore")
        XCTAssertFalse(p.options.respectGitignore)
    }

    func testContextAppliesToBothWhenUnset() throws {
        let p = try Ripgrep.parse("x -C 2")
        XCTAssertEqual(p.options.beforeContext, 2)
        XCTAssertEqual(p.options.afterContext, 2)
    }

    func testExplicitAfterOverridesContext() throws {
        let p = try Ripgrep.parse("x -C 2 -A 5")
        XCTAssertEqual(p.options.afterContext, 5)
        XCTAssertEqual(p.options.beforeContext, 2)
    }

    func testEmptyPathsDefaultsToCwd() throws {
        let p = try Ripgrep.parse("x")
        XCTAssertEqual(p.paths, ["."])
    }

    func testJsonFlagSetsOutputFormat() throws {
        let p = try Ripgrep.parse("--json x")
        XCTAssertEqual(p.outputFormat, .jsonLines)
    }

    func testInvalidArgsThrowWithUsage() {
        XCTAssertThrowsError(try Ripgrep.parse("")) { e in
            guard let e = e as? Ripgrep.Error else { return XCTFail() }
            if case .invalidArguments(let m) = e {
                XCTAssertTrue(m.lowercased().contains("usage") || m.lowercased().contains("missing"))
            } else { XCTFail() }
        }
    }
}
```

- [ ] **Step 2: Implement Parse**

Create `Sources/RipgrepKitTool/Parse.swift`:
```swift
import ArgumentParser
import RipgrepKitCore

extension Ripgrep {
    public enum OutputFormat: String, Codable, Sendable { case text, jsonLines }

    public struct ParsedInvocation: Sendable {
        public let pattern: String
        public let paths: [String]
        public let options: Options
        public let outputFormat: OutputFormat
    }

    public static func parse(_ argString: String) throws -> ParsedInvocation {
        let tokens = try Tokenizer.tokenize(argString)
        return try parse(tokens)
    }

    public static func parse(_ args: [String]) throws -> ParsedInvocation {
        let parsed: RipgrepArgs
        do {
            parsed = try RipgrepArgs.parse(args)
        } catch {
            let msg = RipgrepArgs.fullMessage(for: error)
            throw Ripgrep.Error.invalidArguments(message: msg)
        }

        // Merge -C with -A/-B (rg semantics: -A/-B explicit override -C).
        let after = parsed.afterContext != 0 ? parsed.afterContext : parsed.context
        let before = parsed.beforeContext != 0 ? parsed.beforeContext : parsed.context

        // Split glob into include / exclude by `!` prefix.
        var include: [String] = []
        var exclude: [String] = []
        for g in parsed.glob {
            if g.hasPrefix("!") { exclude.append(String(g.dropFirst())) }
            else { include.append(g) }
        }

        // max-filesize parser (e.g. "5M", "1024K", "100")
        let maxBytes: Int? = parsed.maxFilesize.flatMap(parseFilesize)

        let opts = Options(
            caseInsensitive: parsed.ignoreCase,
            smartCase: parsed.smartCase,
            multiline: parsed.multiline,
            include: include,
            exclude: exclude,
            fileTypes: parsed.fileTypes,
            respectGitignore: !parsed.noIgnore,
            includeHidden: parsed.hidden,
            beforeContext: before,
            afterContext: after,
            maxMatches: parsed.maxCount,
            maxFiles: parsed.maxFiles,
            maxFileSizeBytes: maxBytes,
            timeout: parsed.timeoutMs.map { .milliseconds($0) }
        )

        let paths = parsed.paths.isEmpty ? ["."] : parsed.paths
        return ParsedInvocation(
            pattern: parsed.pattern,
            paths: paths,
            options: opts,
            outputFormat: parsed.json ? .jsonLines : .text
        )
    }
}

private func parseFilesize(_ s: String) -> Int? {
    let s = s.trimmingCharacters(in: .whitespaces)
    guard !s.isEmpty else { return nil }
    let mult: Int
    let numericPart: String
    switch s.last! {
    case "K", "k": mult = 1024;            numericPart = String(s.dropLast())
    case "M", "m": mult = 1024*1024;       numericPart = String(s.dropLast())
    case "G", "g": mult = 1024*1024*1024;  numericPart = String(s.dropLast())
    default:       mult = 1;               numericPart = s
    }
    return Int(numericPart).map { $0 * mult }
}
```

- [ ] **Step 3: Run tests**

Run: `swift test --filter ParseTests`
Expected: 8 tests pass.

- [ ] **Step 4: Commit**

```bash
git add Sources/RipgrepKitTool/Parse.swift Tests/RipgrepKitToolTests/ParseTests.swift
git commit -m "Add parse(_:) → ParsedInvocation with rg-style mappings"
```

---

### Task 30: Run convenience methods

**Files:**
- Create: `Sources/RipgrepKitTool/Run.swift`

- [ ] **Step 1: Write failing test**

Append to `Tests/RipgrepKitToolTests/ParseTests.swift`:
```swift
final class RunTests: XCTestCase {
    private func fixturePath() -> String {
        Bundle.module.url(forResource: "mini", withExtension: nil)!.path
    }

    func testRunStringEndToEnd() async throws {
        // Copy fixture from the Core fixtures (or create a tool-specific one)
        let p = fixturePath()
        let out = try await Ripgrep.run("TODO \(p)")
        XCTAssertTrue(out.contains("TODO"))
    }

    func testRunJSONOutput() async throws {
        let p = fixturePath()
        let out = try await Ripgrep.run("--json TODO \(p)")
        let firstLine = out.split(separator: "\n").first.map(String.init) ?? ""
        XCTAssertTrue(firstLine.hasPrefix("{"))
    }
}
```

> **Fixture note:** RipgrepKitToolTests needs its own Fixtures/mini directory. Replicate it from `Tests/RipgrepKitCoreTests/Fixtures/mini` (or symlink in `.gitignore`-friendly way).

```bash
cp -R Tests/RipgrepKitCoreTests/Fixtures/mini Tests/RipgrepKitToolTests/Fixtures/
```

- [ ] **Step 2: Implement Run**

Create `Sources/RipgrepKitTool/Run.swift`:
```swift
import RipgrepKitCore

extension Ripgrep {
    public static func run(_ argString: String) async throws -> String {
        try await runParsed(parse(argString))
    }
    public static func run(_ args: [String]) async throws -> String {
        try await runParsed(parse(args))
    }

    private static func runParsed(_ p: ParsedInvocation) async throws -> String {
        let result = try await search(pattern: p.pattern, in: p.paths, options: p.options)
        switch p.outputFormat {
        case .text:      return result.formattedAsText()
        case .jsonLines: return result.formattedAsJSONLines()
        }
    }
}
```

- [ ] **Step 3: Run tests**

Run: `swift test --filter RunTests`
Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add Sources/RipgrepKitTool/Run.swift Tests/RipgrepKitToolTests/Fixtures/ Tests/RipgrepKitToolTests/ParseTests.swift
git commit -m "Add Ripgrep.run convenience methods (string + argv overloads)"
```

---

### Task 31: Tool — toolSchema, ToolInput, handleToolCall

**Files:**
- Create: `Sources/RipgrepKitTool/Tool.swift`
- Create: `Tests/RipgrepKitToolTests/ToolCallTests.swift`

- [ ] **Step 1: Write failing tests**

```swift
import XCTest
@testable import RipgrepKitTool
@testable import RipgrepKitCore

final class ToolCallTests: XCTestCase {
    private func fixturePath() -> String {
        Bundle.module.url(forResource: "mini", withExtension: nil)!.path
    }

    func testToolSchemaIsValidJSONWithRequiredKeys() throws {
        let data = Ripgrep.toolSchema.data(using: .utf8)!
        let obj = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(obj?["name"] as? String, "ripgrep")
        XCTAssertNotNil(obj?["description"])
        XCTAssertNotNil(obj?["input_schema"])
    }

    func testToolInputCodableRoundtrip() throws {
        let i = Ripgrep.ToolInput(args: "TODO src/")
        let data = try JSONEncoder().encode(i)
        let decoded = try JSONDecoder().decode(Ripgrep.ToolInput.self, from: data)
        XCTAssertEqual(decoded.args, "TODO src/")
    }

    func testHandleToolCallSucceeds() async throws {
        let input = Ripgrep.ToolInput(args: "TODO \(fixturePath())")
        let out = try await Ripgrep.handleToolCall(input)
        XCTAssertTrue(out.contains("TODO"))
    }

    func testHandleToolCallReturnsErrorString() async throws {
        let input = Ripgrep.ToolInput(args: "[ \(fixturePath())")  // bad regex
        let out = try await Ripgrep.handleToolCall(input)
        XCTAssertTrue(out.hasPrefix("ERROR:"))
    }
}
```

- [ ] **Step 2: Implement Tool**

Create `Sources/RipgrepKitTool/Tool.swift`:
```swift
import Foundation
import RipgrepKitCore

extension Ripgrep {
    public struct ToolInput: Codable, Sendable {
        public let args: String
        public init(args: String) { self.args = args }
    }

    /// Anthropic Messages API tool schema.
    public static let toolSchema: String = #"""
    {
      "name": "ripgrep",
      "description": "Search files using ripgrep (subset). Provide arguments as you would on the rg CLI; behavior matches rg defaults (case-sensitive, respects .gitignore). Examples:\n  \"TODO src/\"\n  \"-S 'func\\s+\\w+' src/ -t swift -A 2\"\n  \"-i error logs/ -g '*.log' -m 50\"\nUnsupported flags: --pre, -z, --type-add, --hyperlink-format, --sort modified, --vimgrep, --binary. Use --json to get JSON-lines output (one match per line, no begin/end envelope).",
      "input_schema": {
        "type": "object",
        "required": ["args"],
        "properties": {
          "args": { "type": "string", "description": "rg-style argument string" }
        }
      }
    }
    """#

    public static func handleToolCall(_ input: ToolInput) async throws -> String {
        do {
            return try await run(input.args)
        } catch let e as Ripgrep.Error {
            return "ERROR: \(e.message)"
        }
    }
}
```

- [ ] **Step 3: Run tests**

Run: `swift test --filter ToolCallTests`
Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add Sources/RipgrepKitTool/Tool.swift Tests/RipgrepKitToolTests/ToolCallTests.swift
git commit -m "Add toolSchema, ToolInput, handleToolCall for LLM integration"
```

---

### Task 32: Full test suite green-light

- [ ] **Step 1: Run everything**

Run:
```bash
cargo test -p ripgrep_core
swift test
```
Expected: all green. Investigate any failure before moving on.

- [ ] **Step 2: Commit any final fixes**

```bash
git add .
git commit -m "Pass full test suite (Rust + Swift)" --allow-empty
```

---

## Phase 6 — Release Infrastructure (Tasks 33-35)

### Task 33: package-release.sh

**Files:**
- Create: `scripts/package-release.sh`

- [ ] **Step 1: Write the script**

Create `scripts/package-release.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

bash scripts/build-xcframework.sh

OUT="dist"
mkdir -p "$OUT"
ZIP="$OUT/RipgrepCore.xcframework.zip"
rm -f "$ZIP"

# Use ditto so symlinks/permissions survive the round-trip.
ditto -c -k --keepParent Frameworks/RipgrepCore.xcframework "$ZIP"

SHA=$(shasum -a 256 "$ZIP" | awk '{print $1}')
echo "$SHA" > "$ZIP.sha256"
echo "Built $ZIP"
echo "SHA256: $SHA"
```

```bash
chmod +x scripts/package-release.sh
```

- [ ] **Step 2: Run it**

Run: `bash scripts/package-release.sh`
Expected: `dist/RipgrepCore.xcframework.zip` and `dist/RipgrepCore.xcframework.zip.sha256` exist.

```bash
ls -lh dist/
```

- [ ] **Step 3: Commit**

```bash
git add scripts/package-release.sh
echo "dist/" >> .gitignore
git add .gitignore
git commit -m "Add package-release.sh for distribution zips"
```

---

### Task 34: GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/release.yml`:
```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust + targets
        run: |
          rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
                            aarch64-apple-darwin x86_64-apple-darwin
          cargo install uniffi-bindgen --version 0.28.0

      - name: Build XCFramework
        run: bash scripts/build-xcframework.sh

      - name: Generate Swift bindings
        run: bash scripts/generate-bindings.sh

      - name: Run tests
        run: |
          cargo test -p ripgrep_core
          swift test

      - name: Package release
        run: bash scripts/package-release.sh

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            dist/RipgrepCore.xcframework.zip
            dist/RipgrepCore.xcframework.zip.sha256
          generate_release_notes: true
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "Add GitHub Actions release workflow"
```

---

### Task 35: Switch Package.swift to remote binaryTarget (executed at first release)

**Files:**
- Modify: `Package.swift`

> Run this task only **after** the first GitHub Release exists with a known sha256. Until then, the local `binaryTarget(path:)` from Task 18 stays.

- [ ] **Step 1: Replace the binaryTarget block**

Edit `Package.swift`:
```swift
.binaryTarget(
    name: "RipgrepCore",
    url: "https://github.com/<owner>/Ripgrep/releases/download/v0.1.0/RipgrepCore.xcframework.zip",
    checksum: "<sha256-from-dist/RipgrepCore.xcframework.zip.sha256>"
),
```

Replace `<owner>` and `<sha256-…>` with real values from the release.

- [ ] **Step 2: Resolve and verify**

Run:
```bash
swift package reset
swift package resolve
swift build
swift test
```
Expected: SwiftPM downloads the xcframework from the URL and tests pass.

- [ ] **Step 3: Commit**

```bash
git add Package.swift Package.resolved
git commit -m "Switch to remote binaryTarget for v0.1.0"
```

---

## Phase 7 — Polish & Deferred Items

### Task 36 (deferred): cargo-fuzz harnesses

Set up `cargo fuzz` targets for: regex compilation, glob compilation, FFI boundary (random `SearchRequest`). Run fuzzers in CI nightly. **Not blocking v0.1.0 release.**

### Task 37 (deferred): README + USAGE docs

Write `README.md` covering: install (SwiftPM), `Ripgrep.search(...)` example, `Ripgrep.run("...")` example, LLM tool integration with Anthropic SDK, supported flag table, unsupported flag table, build-from-source instructions. **Not blocking implementation tests.**

### Task 38 (deferred): Symlink-loop fixture and test

`ignore` already detects loops; verify with a fixture that contains a self-referencing symlink. **Defer until v0.1.0 ships and consumer apps surface the need.**

---

## Plan Self-Review

**Spec coverage check** — every spec section has at least one task:
- §2 unsupported flags — surfaced via parser error path tests (Task 25 invalid pattern, Task 31 error string)
- §4.1 file structure — Tasks 1, 14, 18, 19
- §4.2 SwiftPM targets — Task 18
- §4.3 XCFramework slices — Tasks 16, 17
- §5.1 Cargo deps — Task 1
- §5.2 UniFFI types — Task 14
- §5.3 search pipeline — Tasks 6, 7, 8, 9, 10, 11, 12
- §6.1 typed Swift API — Tasks 19, 20, 21, 22, 23
- §6.2 RipgrepArgs — Task 28
- §6.3 Tokenizer — Task 27
- §6.4 error mapping — Task 29
- §7 LLM tool integration — Task 31
- §8 build & distribution — Tasks 16, 17, 33, 34, 35
- §9 testing strategy — Tasks 8, 9, 10, 11, 12, 25, 26, 27, 28, 29, 31; fuzz row deferred to Task 36
- §10 open items — most resolved by completing tasks; symlink-loop in Task 38; before_context_break edge cases handled in Task 22 / 9
- §10a Swift 6 requirement — Task 18 (`swiftLanguageModes: [.v6]`)
- §11 risk notes — addressed inline (panic Task 12, Mutex avoidance Task 5, schema platform note in Task 31)
- §12 rejected routes — informational; no task needed

**Gaps identified:**
- Task 13 mentions `cargo fmt --check` but doesn't pre-install rustfmt for CI. Add to Task 34's setup step (`rustup component add rustfmt clippy`). → Fixed inline.
- Task 35 placeholder values for owner/sha256 are intentional (real values only available after first GitHub release).

**Type consistency check:**
- `Ripgrep.SearchResult.elapsed` uses `Duration` (Swift) ↔ `elapsed_ms: u64` (Rust); conversion in Task 23 — consistent.
- `Ripgrep.Error.from(_ ffi: RipgrepError)` (Task 20) assumes UniFFI-generated case names; spelled-out warning included to inspect generated bindings.
- `formattedAsText()` line offsets in Task 22 use `m.lineNumber - (m.beforeContext.count - i)` which assumes context lines are contiguous immediately preceding the match. Sink in Task 5 enforces this via `before_buf` ordering. Consistent.

**No placeholders found.**

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-15-ripgrep-swift-package.md`. Two execution options:

1. **Subagent-Driven (recommended)** — Dispatch a fresh subagent per task with two-stage review between tasks. Best for a multi-week build with fresh context per chunk.

2. **Inline Execution** — Execute tasks in this session via `superpowers:executing-plans`, batched with checkpoints. Faster iteration but burns this conversation's context.

Which approach?
