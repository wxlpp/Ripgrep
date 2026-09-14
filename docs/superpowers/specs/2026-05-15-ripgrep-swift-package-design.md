> **Renamed 2026-09-14:** RipgrepKit is now oh-my-grep (package `OhMyGrep`, crate `ohmygrep_core`, FFI module `OhMyGrepCoreFFI`). This document is historical and keeps the original names.

# Ripgrep Swift Package — Design Spec

**Date:** 2026-05-15
**Status:** Approved for planning (revised after Codex adversarial review)
**Owner:** Evan

## 1. Goal

A Swift Package (SwiftPM) named `RipgrepKit` that wraps a curated **subset** of ripgrep 15.x's functionality and exposes a search entry point usable from any iOS or macOS app. The primary consumer is an LLM tool-call: the model emits a `rg`-style argument string, the Swift host parses it, runs the search in-process, and returns formatted results.

## 2. Non-Goals (v1 unsupported / explicit subset)

This is **rg-inspired**, not full rg parity. Unsupported in v1:

- **No CLI executable** (library only; iOS sandbox forbids subprocess).
- **No PCRE2** (avoids the PCRE2 system dependency; default Rust regex is sufficient).
- **No Linux/Windows** (iOS + macOS only via XCFramework).
- **No preprocessor** (`--pre`, `--pre-glob`).
- **No compressed file search** (`-z`, `--search-zip`).
- **No stdin input** (search operates on file paths).
- **No custom encoding / BOM sniffing** (UTF-8 lossy only; non-UTF-8 bytes become U+FFFD).
- **No `--type-add` / `--type-clear`** (only ripgrep's built-in default type definitions).
- **No mtime sort** (`--sort modified`, `--sortr`); results are sorted deterministically by `(path, line_number)`.
- **No terminal hyperlinks** (`--hyperlink-format`).
- **No special binary modes** (`--binary`, `-a`); binary files are skipped per `ignore`'s default detection.
- **No `--null-data`, `--passthru`, `--vimgrep`** output modes; only plain text and JSON-lines.

Unsupported flags surface as `Ripgrep.Error.invalidArguments(message:)` with a descriptive message, so the LLM can self-correct.

## 3. Constraints

- Must run on **iOS device, iOS simulator, and macOS** → in-process linkage, no subprocess.
- Search semantics derived from ripgrep's official sub-crates (`ignore`, `grep-*`, `globset`) so behavior matches user expectations within the supported subset.
- Tool-call schema is **single-string** (`{ "args": "..." }`) so LLM can rely on its `rg` CLI knowledge.
- **Cancellation is v1**: long-running searches must respect Swift `Task.cancel()` so they don't block agent loops.

## 4. Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Consumer app (iOS / macOS)                              │
│   try await Ripgrep.run("'TODO|FIXME' src/ -t swift")   │
└────────────────────┬────────────────────────────────────┘
                     │
        ┌────────────▼─────────────┐
        │ RipgrepKitTool (Swift)   │  swift-argument-parser
        │ - tokenizer              │  + LLM tool helpers
        │ - RipgrepArgs (Parsable) │  + output formatters
        │ - parse / format         │
        └────────────┬─────────────┘
                     │ uses typed API
        ┌────────────▼─────────────┐
        │ RipgrepKitCore (Swift)   │  Public typed API
        │ - Ripgrep.search(...)    │  - Options / Match / Result
        │ - Codable + Sendable     │  - error mapping
        └────────────┬─────────────┘
                     │ Codable structs over UniFFI
        ┌────────────▼─────────────┐
        │ RipgrepKitFFI (Swift)    │  uniffi-bindgen output
        └────────────┬─────────────┘
                     │ FFI (C ABI), sync entry + cancel token
        ┌────────────▼─────────────┐
        │ RipgrepCore.xcframework  │  Static lib (Rust)
        │   ios-arm64              │  ignore + grep-regex
        │   ios-arm64_x86_64-sim   │  + grep-searcher + globset
        │   macos-arm64_x86_64     │  + AtomicBool cancel +
        │                          │    catch_unwind at FFI edge
        └──────────────────────────┘
```

### 4.1 Repository Layout

```
Ripgrep/
├── Package.swift                       # SwiftPM manifest
├── Cargo.toml                          # Rust workspace root
├── crates/
│   └── ripgrep_core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # UniFFI exports + catch_unwind
│           ├── search.rs               # WalkParallel + grep-searcher
│           ├── cancel.rs               # CancelToken (AtomicBool wrapper)
│           ├── options.rs              # Request / Match / Result
│           └── error.rs                # RipgrepError
├── Sources/
│   ├── RipgrepKitFFI/                  # uniffi-bindgen output (committed)
│   │   └── ripgrep_core.swift
│   ├── RipgrepKitCore/                 # Typed API only — NO argument-parser dep
│   │   ├── Ripgrep.swift               # namespace + search(...)
│   │   ├── Options.swift
│   │   ├── SearchResult.swift          # types + formatters
│   │   └── Error.swift
│   └── RipgrepKitTool/                 # CLI parsing + LLM tool helpers
│       ├── RipgrepArgs.swift           # ParsableCommand
│       ├── Tokenizer.swift             # Shell-style argv splitter
│       ├── Parse.swift                 # parse(_:) -> ParsedInvocation
│       ├── Run.swift                   # run(_:) convenience over parse+search+format
│       └── Tool.swift                  # toolSchema, ToolInput, handleToolCall
├── scripts/
│   ├── build-xcframework.sh            # Builds 5 Rust targets, lipos, packages
│   ├── generate-bindings.sh            # Runs uniffi-bindgen swift
│   ├── package-release.sh              # Zips xcframework + computes SHA256
│   └── ci.sh
├── Tests/
│   ├── RipgrepKitCoreTests/
│   │   ├── Fixtures/
│   │   ├── SearchTests.swift
│   │   ├── CancellationTests.swift
│   │   └── ErrorTests.swift
│   └── RipgrepKitToolTests/
│       ├── TokenizerTests.swift
│       ├── ArgsParsingTests.swift
│       ├── ParseTests.swift
│       └── ToolCallTests.swift
├── fixture/                            # Sample data for end-to-end tests
└── docs/superpowers/specs/             # This document
```

### 4.2 Package.swift Targets

```swift
let package = Package(
    name: "RipgrepKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    // swiftLanguageVersions set elsewhere in the manifest; v6 required (see §10a).
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
            url: "https://github.com/<owner>/Ripgrep/releases/download/<tag>/RipgrepCore.xcframework.zip",
            checksum: "<sha256>"
        ),
        .target(name: "RipgrepKitFFI", dependencies: ["RipgrepCore"]),
        .target(name: "RipgrepKitCore", dependencies: ["RipgrepKitFFI"]),
        .target(name: "RipgrepKitTool", dependencies: [
            "RipgrepKitCore",
            .product(name: "ArgumentParser", package: "swift-argument-parser"),
        ]),
        .testTarget(name: "RipgrepKitCoreTests",
                    dependencies: ["RipgrepKitCore"],
                    resources: [.copy("Fixtures")]),
        .testTarget(name: "RipgrepKitToolTests",
                    dependencies: ["RipgrepKitTool"],
                    resources: [.copy("Fixtures")]),
    ]
)
```

Consumers of just the typed API depend on `RipgrepKitCore` and pay no `argument-parser` cost. Consumers wiring up an LLM tool depend on `RipgrepKitTool`.

### 4.3 XCFramework Slices

| XCFramework slice | Rust target(s) | Notes |
|---|---|---|
| `ios-arm64` | `aarch64-apple-ios` | Device |
| `ios-arm64_x86_64-simulator` | `aarch64-apple-ios-sim` + `x86_64-apple-ios` | lipo'd fat |
| `macos-arm64_x86_64` | `aarch64-apple-darwin` + `x86_64-apple-darwin` | lipo'd fat |

**3 XCFramework slices, 5 Rust target triples (single `ripgrep_core` cargo target).** Distributed via GitHub Release as `RipgrepCore.xcframework.zip` with SHA256 checksum committed in `Package.swift`.

## 5. Rust Core (`ripgrep_core`)

### 5.1 Cargo Dependencies

```toml
[dependencies]
ignore = "0.4"
grep-regex = "0.1"
grep-searcher = "0.1"
globset = "0.4"
crossbeam-channel = "0.5"     # lock-free MPSC for collecting matches from WalkParallel
uniffi = "0.28"
thiserror = "2"
```

(Versions track ripgrep 15.x's lockfile; pinned exact versions in implementation plan.)

### 5.2 UniFFI-Exported Types

```rust
#[derive(uniffi::Record)]
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
    pub max_matches: Option<u32>,        // global cap on matches across all files
    pub max_files: Option<u32>,          // cap on regular files whose contents are searched
                                         // (directories and skipped binary files are not counted)
    pub max_file_size_bytes: Option<u64>,
    pub timeout_ms: Option<u64>,         // wall-clock cap; deadline-based cancel
}

#[derive(uniffi::Record)]
pub struct Submatch { pub start: u32, pub end: u32 }

#[derive(uniffi::Record)]
pub struct SearchMatch {
    pub path: String,
    pub line_number: u64,
    pub line: String,
    pub before_context: Vec<String>,
    pub after_context: Vec<String>,
    pub submatches: Vec<Submatch>,
}

#[derive(uniffi::Record)]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,                 // hit max_matches or max_files
    pub cancelled: bool,                 // hit timeout or external cancel
    pub files_searched: u64,
    pub elapsed_ms: u64,
}

#[derive(uniffi::Object)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    deadline: Option<Instant>,           // immutable; set at construction
}

#[uniffi::export]
impl CancelToken {
    /// `timeout_ms == None` means no deadline; only explicit `cancel()` will trip the token.
    #[uniffi::constructor]
    pub fn new(timeout_ms: Option<u64>) -> Arc<Self> { /* ... */ }
    pub fn cancel(&self) { /* set flag */ }
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
            || self.deadline.is_some_and(|d| Instant::now() >= d)
    }
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum RipgrepError {
    #[error("invalid regex: {0}")] InvalidPattern(String),
    #[error("path not found: {0}")] PathNotFound(String),
    #[error("io error: {0}")]       Io(String),
    #[error("internal panic: {0}")] InternalPanic(String),
}

/// Synchronous entry point. Long-running, must be called off the main thread
/// (Swift wraps in Task.detached). Honors cancel token.
#[uniffi::export]
pub fn search_blocking(
    request: SearchRequest,
    cancel: Option<Arc<CancelToken>>,
) -> Result<SearchResult, RipgrepError>;
```

**Why sync (not `async fn`):** UniFFI's async support runs the future on its tokio runtime; a CPU/IO-blocking search would starve runtime workers. Exposing a sync function lets Swift control the threading via `Task.detached { ... }` (or a custom executor). Cancellation flows through the explicit `CancelToken` — Swift's `withTaskCancellationHandler` sets the flag.

### 5.3 Search Pipeline

```
search_blocking(req, cancel)
  ① catch_unwind { ... } — translate any panic to RipgrepError::InternalPanic
  ② Build RegexMatcher (case_smart / multi_line)
  ③ Build ignore::WalkBuilder:
       hidden(!include_hidden), git_ignore(respect_gitignore),
       git_global(respect_gitignore), git_exclude(respect_gitignore),
       types(TypesBuilder for file_types),
       max_filesize(max_file_size_bytes),
       overrides(OverrideBuilder for include/exclude globs)
  ④ CancelToken constructed with deadline derived from req.timeout_ms (if any).
     Caller may also share their own CancelToken (Swift wires Task.cancel()).
  ⑤ Create crossbeam_channel::unbounded::<SearchMatch>() (lock-free MPSC).
     WalkParallel.run with shared:
       - AtomicUsize match counter, AtomicUsize file counter
       - cancel token (checked at every dir entry → WalkState::Quit if tripped)
       - sender end of the channel
  ⑥ Per file: SearcherBuilder(before/after_context, multi_line) + custom Sink
       Sink::matched / Sink::context push SearchMatch into the channel.
       Sink checks cancel.is_cancelled() at minimum every 100 events
       (matched + context combined); returning Err aborts the current file
       and propagates as WalkState::Quit on the next dir entry.
  ⑦ Drop sender; drain receiver into Vec<SearchMatch>.
     Sort by (path, line_number); truncate at max_matches.
  ⑧ Return SearchResult { matches, truncated, cancelled, ... }
```

**Threading:** `WalkParallel` uses `std::thread`, separate from any Swift/tokio runtime. No nested-runtime concern. The whole `search_blocking` call is meant to run on a dedicated Swift thread (`Task.detached`).

**Panic safety:** Top-level `catch_unwind` at the FFI boundary; worker-thread panics surface through `WalkParallel`'s join, which we capture and convert to `InternalPanic`. No unwinding crosses the C ABI.

**Text encoding:** All bytes → UTF-8 lossy.

## 6. Swift API

### 6.1 `RipgrepKitCore` — Typed API

```swift
public enum Ripgrep {
    public struct Options: Codable, Sendable {
        public var caseInsensitive: Bool = false
        public var smartCase: Bool = false       // matches rg CLI default
        public var multiline: Bool = false
        public var include: [String] = []
        public var exclude: [String] = []
        public var fileTypes: [String] = []
        public var respectGitignore: Bool = true
        public var includeHidden: Bool = false
        public var beforeContext: Int = 0
        public var afterContext: Int = 0
        public var maxMatches: Int? = nil
        public var maxFiles: Int? = nil
        public var maxFileSizeBytes: Int? = nil
        public var timeout: Duration? = nil
        public init(...) { ... }
    }

    public struct Match: Codable, Sendable {
        public let path: String
        public let lineNumber: Int
        public let line: String
        public let beforeContext: [String]
        public let afterContext: [String]
        public let submatches: [Submatch]
    }

    public struct Submatch: Codable, Sendable {
        public let start: Int
        public let end: Int
    }

    public struct SearchResult: Codable, Sendable {
        public let matches: [Match]
        public let truncated: Bool
        public let cancelled: Bool
        public let filesSearched: Int
        public let elapsed: Duration              // FFI carries u64 ms; Swift exposes Duration

        public func formattedAsText() -> String        // see format spec below
        public func formattedAsJSONLines() -> String   // per-match JSON objects only
    }

    public enum Error: Swift.Error, Sendable {
        case invalidArguments(message: String)         // from RipgrepKitTool
        case invalidPattern(String)
        case pathNotFound(String)
        case io(String)
        case internalPanic(String)

        public var message: String { /* human-readable */ }
    }

    /// Runs the search on a detached task. Honors `Task.cancel()` via a
    /// CancelToken bridged from withTaskCancellationHandler.
    public static func search(
        pattern: String,
        in paths: [String],
        options: Options = .init()
    ) async throws -> SearchResult
}
```

**Cancellation wiring** (sketch):
```swift
public static func search(...) async throws -> SearchResult {
    let timeoutMs = options.timeout.map { UInt64($0.components.seconds * 1000) }
    let token = CancelToken(timeoutMs: timeoutMs)
    return try await withTaskCancellationHandler {
        try await Task.detached(priority: .userInitiated) {
            try Ripgrep.searchBlocking(request: request.toFFI(), cancel: token)
        }.value
    } onCancel: {
        token.cancel()
    }
}
```

**Bounds checks at FFI boundary** (M5 — Int → u32 wrap-around safety):
`Options.toFFI()` calls `precondition(value >= 0, "...")` for every `Int` field that maps to a Rust unsigned type (`beforeContext`, `afterContext`, `maxMatches`, `maxFiles`, `maxFileSizeBytes`). A negative value would otherwise wrap to `u32::MAX` / `u64::MAX` and cause OOM-class behavior inside the walker.

**`formattedAsText()` output spec** (m3):
Mirrors `rg`'s grouped default output — context lines use `-` separator instead of `:`, blank line separates per-file groups (no separator between same-file matches), no leading file header.

```text
src/foo.swift-8-let x = 1
src/foo.swift:10:    // TODO: rename
src/foo.swift-12-let z = 3

src/bar.rs:5:fn fixme() {
src/bar.rs-6-    // TODO: handle error
```

When matches and context lines for the same file are interleaved, ordering follows `(path, lineNumber)` deterministically; rule for omitting `--` between two matches with no overlap deferred to implementation (mirror rg's `before_context_break` behavior).

### 6.2 `RipgrepKitTool` — CLI parsing + LLM helpers

```swift
import RipgrepKitCore
import ArgumentParser

extension Ripgrep {
    public enum OutputFormat: String, Codable, Sendable { case text, jsonLines }

    public struct ParsedInvocation: Sendable {
        public let pattern: String
        public let paths: [String]
        public let options: Options
        public let outputFormat: OutputFormat
    }

    /// Tokenizes and parses an rg-style argument string (or pre-tokenized argv)
    /// into a typed invocation. Throws .invalidArguments on parse failure.
    public static func parse(_ argString: String) throws -> ParsedInvocation
    public static func parse(_ args: [String]) throws -> ParsedInvocation

    /// Convenience: parse + search + format-as-string in one call.
    public static func run(_ argString: String) async throws -> String
    public static func run(_ args: [String]) async throws -> String
}
```

**`RipgrepArgs`** (`ParsableCommand`, flag names match `rg` 1:1):

```swift
struct RipgrepArgs: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "rg",
        abstract: "Search files using ripgrep semantics (subset)."
    )

    @Argument var pattern: String
    @Argument var paths: [String] = []                                    // empty → ["."]

    @Flag(name: [.short, .customLong("ignore-case")])    var ignoreCase = false
    @Flag(name: [.customShort("S"), .customLong("smart-case")])
        var smartCase = false                                             // matches rg default
    @Flag(name: [.customShort("U"), .customLong("multiline")]) var multiline = false

    @Option(name: [.customShort("g"), .customLong("glob")])    var glob: [String] = []
    @Option(name: [.customShort("t"), .customLong("type")])    var fileTypes: [String] = []

    @Flag(name: .customLong("hidden"))     var hidden = false
    @Flag(name: .customLong("no-ignore"))  var noIgnore = false

    @Option(name: [.customShort("A"), .customLong("after-context")])  var afterContext = 0
    @Option(name: [.customShort("B"), .customLong("before-context")]) var beforeContext = 0
    @Option(name: [.customShort("C"), .customLong("context")])        var context = 0

    @Option(name: [.customShort("m"), .customLong("max-count")])      var maxCount: Int?
    @Option(name: .customLong("max-files"))                           var maxFiles: Int?
    @Option(name: .customLong("max-filesize"))                        var maxFilesize: String?
    @Option(name: .customLong("timeout-ms"))                          var timeoutMs: Int?

    @Flag(name: .customLong("json"))       var json = false              // selects output format

    func run() throws { /* unused */ }
}
```

**Notable mappings (`toParsed`):**
- `respectGitignore` ← `!noIgnore`
- `context` (-C) sets both before/after only when they weren't set explicitly
- Empty `paths` → `["."]`
- `glob` array splits by `!` prefix into `include` vs `exclude`
- `--json` → `outputFormat = .jsonLines`, otherwise `.text`
- Default invocation in `Options` mirrors `RipgrepArgs` defaults exactly (notably `smartCase: false`, `respectGitignore: true`)

### 6.3 Tokenizer

`func tokenize(_ s: String) throws(Ripgrep.Error) -> [String]` — minimal shell-style splitter (Swift 6 typed throws so callers get static guarantees the only error type is `Ripgrep.Error`).

- Splits on unquoted whitespace
- Honors single quotes (literal), double quotes (with `\` escapes), backslash escapes outside quotes
- **`--flag=value` semantics:** the `=` belongs to the same token as `--flag`. Anything after `=` (until the next unquoted whitespace) is the *value portion* of the same token; quotes inside the value portion are processed (stripped, escapes applied) but `=` does **not** trigger re-splitting. So `--glob='*.swift'` becomes the single token `--glob=*.swift`; `--glob="hello world"` becomes `--glob=hello world` (one token).
- **No** variable expansion, command substitution, or glob expansion

Throws `Ripgrep.Error.invalidArguments` on unbalanced quotes / dangling escape.

### 6.4 Argument Parser Error Mapping

Catch `ArgumentParser` errors, render with `RipgrepArgs.fullMessage(for:)` (includes usage hint), wrap into `.invalidArguments(message:)`. The message string is what reaches the LLM as `ERROR: ...`, enabling self-correction.

## 7. LLM Tool Integration

The `toolSchema` literal below targets the **Anthropic Messages API tool schema** (top-level `name` / `description` / `input_schema`). Adapters for OpenAI's `function`/`parameters` shape, Google Gemini's `functionDeclarations`, etc., should be derived in consumer code from the same underlying `ToolInput` (Codable). v1 ships only the Anthropic form.

`formattedAsJSONLines()` emits **one JSON object per match, no envelope** — no `begin` / `end` / `summary` records (unlike rg's full `--json` schema). LLMs parse line-by-line trivially; envelope records add token cost without adding signal here.

```swift
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
            let parsed = try parse(input.args)
            let result = try await search(
                pattern: parsed.pattern, in: parsed.paths, options: parsed.options
            )
            switch parsed.outputFormat {
            case .text:      return result.formattedAsText()
            case .jsonLines: return result.formattedAsJSONLines()
            }
        } catch let e as Ripgrep.Error {
            return "ERROR: \(e.message)"
        }
    }
}
```

**Output format selection** is now driven by `parsed.outputFormat`, set by the `--json` flag at parse time. No tuple returns or thread-locals.

**Typical consumer code:**

```swift
case .toolUse(let block) where block.name == "ripgrep":
    let input = try block.input.decode(as: Ripgrep.ToolInput.self)
    let output = try await Ripgrep.handleToolCall(input)
    sendToolResult(output)
```

## 8. Build & Distribution

### 8.1 Developer Workflow (rebuild xcframework locally)

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
                  aarch64-apple-darwin x86_64-apple-darwin
./scripts/build-xcframework.sh           # ~3-5 min on Apple Silicon
./scripts/generate-bindings.sh           # writes Sources/RipgrepKitFFI/ripgrep_core.swift
./scripts/package-release.sh             # zips to RipgrepCore.xcframework.zip + sha256

# Point Package.swift at a local file:// URL during dev:
.binaryTarget(name: "RipgrepCore",
              path: "Frameworks/RipgrepCore.xcframework")   // dev override
```

A `Package.swift.dev` (or a `RIPGREP_LOCAL_BINARY=1` env-driven branch) lets contributors swap to a local `path:`-style binaryTarget without committing the URL form.

### 8.2 build-xcframework.sh

For each Rust target:
1. `cargo build --release --target <triple> -p ripgrep_core`
2. lipo simulator/macOS slices into fat binaries
3. Stage each into a `.framework` dir with `Info.plist` + `module.modulemap`
4. `xcodebuild -create-xcframework -framework ... -output ./build/RipgrepCore.xcframework`

### 8.3 Release Workflow (CI)

`.github/workflows/release.yml` (macOS runner) on tag push:
1. Build xcframework
2. Run all tests against locally-built xcframework
3. `package-release.sh` → `RipgrepCore.xcframework.zip` + `RipgrepCore.xcframework.zip.sha256`
4. Publish GitHub Release with the zip + checksum file
5. Bot opens a follow-up PR updating `Package.swift`'s `url:` and `checksum:` to the new release

Consumers then resolve via SwiftPM with no Rust toolchain required.

## 9. Testing Strategy

| Layer | Tool | What |
|---|---|---|
| Rust core | `cargo test` | Walker filtering, gitignore, context lines, max_matches/max_files truncation, multiline, file types, **cancellation flag honored mid-walk**, **panic in worker → InternalPanic** |
| Tokenizer | XCTest | Single/double quotes, mixed, backslash escapes, `--flag=value`, empty args, dangling escapes, Windows-ish paths, mixed whitespace |
| RipgrepArgs | XCTest | Each flag's short/long forms, `--no-smart-case` inverse, conflict combos, defaults exactly mirror rg |
| End-to-end (Core) | XCTest | `Ripgrep.search(...)` against fixture mini-repo |
| End-to-end (Tool) | XCTest | `Ripgrep.run("...")`, `parse(...)`, `handleToolCall(...)` |
| Cancellation | XCTest | `Task { try await search(...) }` then `task.cancel()` returns within ~50ms with `cancelled: true` |
| Panic safety | XCTest | Inject test-only Rust panic, assert `.internalPanic(_)` is thrown, no crash |
| Tool roundtrip | XCTest | JSON-encoded `ToolInput` → `handleToolCall` → assert text/JSON shape |
| Error path | XCTest | Bad regex, missing path, malformed args, unsupported flag → message contains usage hint |
| Fuzz | `cargo-fuzz` | Regex compilation, glob compilation, FFI boundary (random `SearchRequest` fields) — no panics, no UB, no unbounded memory |

Fixture repo (≈10 files): `.gitignore`, mix of `.swift`/`.rs`/`.ts`/`.txt`, one hidden file `.env`, one ignored file `target/foo.txt`, one large file (>1MB) for max-filesize tests, one symlink loop for walker robustness.

## 10. Open Items Deferred to Implementation

- Exact crate version pinning (track ripgrep 15.1.0's `Cargo.lock`).
- Whether `CancelToken` lives in Rust as `uniffi::Object` or is reconstructed from a raw pointer per call (UniFFI ergonomics dependent).
- Symlink loop handling (`ignore` has detection; just confirm + test).
- Exact `before_context_break` rule for `formattedAsText()` (when to emit `--` between adjacent matches in the same file). Mirror rg's behavior; test against fixtures.

## 10a. Swift Language Requirement

`Package.swift` declares `swiftLanguageVersions: [.v6]`. Required for:
- Typed throws (`throws(Ripgrep.Error)`) used by the tokenizer / parser
- Strict `Sendable` checking on `Options` / `SearchResult`

This raises the toolchain floor to Xcode 16 / Swift 6.0+. Consumers on older toolchains can still use `RipgrepKitFFI` directly but lose the typed-throws / Sendable guarantees from the higher layers.

## 11. Risk Notes

- **UniFFI async vs blocking work:** Avoided by exposing sync `search_blocking` and letting Swift control thread placement via `Task.detached`. Cancellation goes through an explicit `CancelToken`, not Tokio's cooperative cancel.
- **Panic safety across FFI:** Mitigated by top-level `catch_unwind` + capturing `WalkParallel` join errors. Worth a fuzz pass at implementation time.
- **`grep-printer` not used:** We collect matches via a custom `Sink`, which is small (≈100 LoC) but the place where context-line edge cases will manifest. Cover thoroughly in tests.
- **swift-argument-parser exit behavior:** `parse()` does not exit the process (only `main()` does). Safe inside a library.
- **XCFramework size:** Estimate 5–10 MB per slice; ~25–35 MB zipped. Document in README.
- **Test fixture symlinks:** Need OS-agnostic creation in tests (not all CI runners preserve them through git).

## 12. Routes Considered and Rejected

| Route | Why rejected |
|---|---|
| **Pure Swift** (NSRegularExpression / `Regex` + `FileManager` + hand-rolled gitignore) | Lower-fidelity than ripgrep on perf and `.gitignore` semantics; reinventing a moving target. Acceptable only if ripgrep parity isn't desired — which it is here. |
| **macOS `Process` + bundled rg binary** | Doesn't work on iOS (sandbox forbids fork/exec). Would force a different code path per platform. |
| **iOS XPC to a host service** | XPC across processes still needs a host process to run rg, which iOS apps can't spawn. Useful only if there's a separate macOS helper, which doesn't exist here. |
| **WASM (wasmer/wasmtime in iOS)** | iOS forbids JIT; AOT-only WASM runtimes work but bring another large dependency, and ripgrep-as-WASM lacks a maintained build with WASI FS access. Higher risk, no real upside vs the static-lib path. |
| **swift-bridge instead of UniFFI** | Smaller community, weaker async/error-type story, manual struct mapping. UniFFI's `#[derive(uniffi::Record)]` covers our needs cleanly. |
| **Commit XCFramework into git history** | 25-50 MB per binary refresh inflates clones, pollutes PR diffs, and complicates merges. GitHub Release + `binaryTarget(url:checksum:)` keeps the source repo lean. |
