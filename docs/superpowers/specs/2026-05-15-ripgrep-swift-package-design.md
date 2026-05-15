# Ripgrep Swift Package — Design Spec

**Date:** 2026-05-15
**Status:** Approved for planning
**Owner:** Evan

## 1. Goal

A Swift Package (SwiftPM) named `RipgrepKit` that wraps ripgrep 15.x and exposes a single search entry point usable from any iOS or macOS app. The primary consumer is an LLM tool-call: the model emits a `rg`-style argument string, the Swift host parses it, runs the search in-process, and returns formatted results.

## 2. Non-Goals

- **No CLI binary**: This is a library, not an executable. (Bundling the `rg` binary + `Process` is impossible on iOS.)
- **No reimplementation of ripgrep**: We compose ripgrep's official sub-crates (`ignore`, `grep-*`, `globset`); we do not fork or rewrite its core.
- **No PCRE2** in v1: omits the heavy PCRE2 system dependency. Default Rust regex covers the LLM tool-call use cases.
- **No Linux/Windows support** in v1: scope is iOS + macOS via XCFramework.
- **No cancellation/timeout API** in v1: bounded by `--max-count`. Forwarding Swift `Task.cancel()` to a Rust atomic flag is deferred to v2.

## 3. Constraints

- Must run on **iOS device, iOS simulator, and macOS** → forces in-process linkage, no subprocess.
- Source of truth for search behavior is **ripgrep's own crates** (so semantics match what users expect from `rg`).
- Tool-call schema must be **single-string** (`{ "args": "..." }`) so LLM can rely on its `rg` CLI knowledge.

## 4. Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Consumer app (iOS / macOS)                              │
│   try await Ripgrep.run("'TODO|FIXME' src/ -t swift")   │
└────────────────────┬────────────────────────────────────┘
                     │
        ┌────────────▼─────────────┐
        │ RipgrepKit (Swift)       │  swift-argument-parser
        │ - tokenizer              │  + typed Options
        │ - RipgrepArgs (Parsable) │  + LLM tool helpers
        │ - SearchResult formatter │
        └────────────┬─────────────┘
                     │ Codable structs over UniFFI
        ┌────────────▼─────────────┐
        │ RipgrepKitFFI (Swift)    │  Auto-generated
        │ uniffi-bindgen output    │  by uniffi-bindgen swift
        └────────────┬─────────────┘
                     │ FFI (C ABI)
        ┌────────────▼─────────────┐
        │ RipgrepCore.xcframework  │  Static lib (Rust)
        │   ios-arm64              │  ignore + grep-regex
        │   ios-arm64_x86_64-sim   │  + grep-searcher + globset
        │   macos-arm64_x86_64     │
        └──────────────────────────┘
```

### 4.1 Repository Layout

```
Ripgrep/
├── Package.swift                       # SwiftPM manifest
├── Cargo.toml                          # Rust workspace root
├── crates/
│   └── ripgrep_core/                   # The Rust crate
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # UniFFI exports
│           ├── search.rs               # WalkParallel + grep-searcher composition
│           ├── options.rs              # Request / Match / Result types
│           └── error.rs                # RipgrepError enum
├── Sources/
│   ├── RipgrepKitFFI/
│   │   └── ripgrep_core.swift          # uniffi-bindgen output (committed)
│   └── RipgrepKit/
│       ├── Ripgrep.swift               # Public API namespace
│       ├── RipgrepArgs.swift           # ParsableCommand definition
│       ├── Tokenizer.swift             # Shell-style argv splitter
│       ├── Options.swift               # Public Options struct
│       ├── SearchResult.swift          # Public result types + formatters
│       └── Tool.swift                  # toolSchema, ToolInput, handleToolCall
├── Frameworks/
│   └── RipgrepCore.xcframework         # Committed binary artifact
├── scripts/
│   ├── build-xcframework.sh            # Builds 5 Rust slices, lipos, packages
│   ├── generate-bindings.sh            # Runs uniffi-bindgen swift
│   └── ci.sh                           # Local equivalent of CI workflow
├── Tests/
│   └── RipgrepKitTests/
│       ├── Fixtures/                   # Mini repo with .gitignore, hidden, etc.
│       ├── TokenizerTests.swift
│       ├── ArgsParsingTests.swift
│       ├── SearchTests.swift
│       └── ToolCallTests.swift
├── fixture/                            # Sample data for end-to-end tests
└── docs/superpowers/specs/             # This document
```

### 4.2 Package.swift Targets

- `.binaryTarget(name: "RipgrepCore", path: "Frameworks/RipgrepCore.xcframework")`
- `.target(name: "RipgrepKitFFI", dependencies: ["RipgrepCore"])`
- `.target(name: "RipgrepKit", dependencies: ["RipgrepKitFFI", .product(name: "ArgumentParser", package: "swift-argument-parser")])`
- `.testTarget(name: "RipgrepKitTests", dependencies: ["RipgrepKit"], resources: [.copy("Fixtures")])`

Dependency: `apple/swift-argument-parser` from 1.5.0.

### 4.3 XCFramework Slices

| Slice | Rust target |
|---|---|
| `ios-arm64` | `aarch64-apple-ios` |
| `ios-arm64_x86_64-simulator` | `aarch64-apple-ios-sim` + `x86_64-apple-ios` (lipo) |
| `macos-arm64_x86_64` | `aarch64-apple-darwin` + `x86_64-apple-darwin` (lipo) |

The xcframework is **committed to the repo** in v1 so consumers don't need a Rust toolchain. Future v2 may switch to remote `.xcframework.zip` via GitHub Releases.

## 5. Rust Core (`ripgrep_core`)

### 5.1 Cargo Dependencies

```toml
[dependencies]
ignore = "0.4"          # Walker + .gitignore + hidden + globset overrides
grep-regex = "0.1"      # Default regex matcher
grep-searcher = "0.1"   # Search loop + context lines + multiline
globset = "0.4"         # Include/exclude glob compilation
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
    pub max_matches: Option<u32>,
    pub max_file_size_bytes: Option<u64>,
}

#[derive(uniffi::Record)]
pub struct Submatch { pub start: u32, pub end: u32 }

#[derive(uniffi::Record)]
pub struct SearchMatch {
    pub path: String,
    pub line_number: u64,
    pub line: String,                 // UTF-8 lossy
    pub before_context: Vec<String>,
    pub after_context: Vec<String>,
    pub submatches: Vec<Submatch>,
}

#[derive(uniffi::Record)]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,
    pub files_searched: u64,
    pub elapsed_ms: u64,
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum RipgrepError {
    #[error("invalid regex: {0}")] InvalidPattern(String),
    #[error("path not found: {0}")] PathNotFound(String),
    #[error("io error: {0}")]       Io(String),
}

#[uniffi::export(async_runtime = "tokio")]
pub async fn search(request: SearchRequest) -> Result<SearchResult, RipgrepError>;
```

### 5.3 Search Pipeline

```
search(req)
  ① Build RegexMatcher with case_smart / multi_line flags
  ② Build ignore::WalkBuilder
       - hidden(!req.include_hidden)
       - git_ignore(req.respect_gitignore)
       - git_global(req.respect_gitignore)
       - git_exclude(req.respect_gitignore)
       - types(TypesBuilder for req.file_types)
       - max_filesize(req.max_file_size_bytes)
       - overrides(OverrideBuilder for include/exclude globs)
  ③ WalkParallel with shared:
       - AtomicUsize match counter
       - AtomicBool stop flag (set when counter ≥ max_matches)
       - Mutex<Vec<SearchMatch>> sink
  ④ Per file: SearcherBuilder with before/after_context, multi_line, runs custom Sink
       - Sink::matched: capture line, lineno, submatch byte ranges
       - Sink::context:  capture context line
  ⑤ Sort results by (path, line_number); truncate at max_matches
  ⑥ Return SearchResult (matches, truncated, files_searched, elapsed_ms)
```

**Text encoding:** All paths and lines are converted from raw bytes via UTF-8 lossy (matches ripgrep's default behavior).

## 6. Swift API (`RipgrepKit`)

### 6.1 Public Surface

```swift
public enum Ripgrep {
    // CLI-string entry point — primary LLM tool path
    public static func run(_ argString: String) async throws -> SearchResult

    // Pre-tokenized argv entry point — programmatic / avoids quoting issues
    public static func run(_ args: [String]) async throws -> SearchResult

    // Typed entry point — for callers that don't want CLI parsing at all
    public static func search(
        pattern: String,
        in paths: [String],
        options: Options = .init()
    ) async throws -> SearchResult

    public struct Options: Codable, Sendable { /* mirrors RipgrepArgs */ }
    public struct Match: Codable, Sendable    { /* mirrors SearchMatch */ }
    public struct Submatch: Codable, Sendable { let start: Int; let end: Int }
    public struct SearchResult: Codable, Sendable {
        public let matches: [Match]
        public let truncated: Bool
        public let filesSearched: Int
        public let elapsedMs: Int
        public func formattedAsText() -> String       // rg-default style
        public func formattedAsJSONLines() -> String  // rg --json style
    }

    public enum Error: Swift.Error, Sendable {
        case invalidArguments(message: String)  // wraps ArgumentParser errors
        case invalidPattern(String)
        case pathNotFound(String)
        case io(String)

        public var message: String { /* human-readable, for LLM retries */ }
    }
}
```

### 6.2 RipgrepArgs (`ParsableCommand`)

Flag names match the real `rg` CLI 1:1 so LLM training-data knowledge transfers directly.

```swift
struct RipgrepArgs: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "rg",
        abstract: "Search files using ripgrep semantics."
    )

    @Argument var pattern: String
    @Argument var paths: [String] = []     // empty → ["."]

    @Flag(name: [.short, .customLong("ignore-case")])     var ignoreCase = false
    @Flag(name: .customLong("smart-case"), inversion: .prefixedNo)
        var smartCase = true   // ← default ON for LLM ergonomics
    @Flag(name: [.customShort("U"), .customLong("multiline")]) var multiline = false

    @Option(name: [.customShort("g"), .customLong("glob")])
        var glob: [String] = []     // include; "!pat" excludes
    @Option(name: [.customShort("t"), .customLong("type")])
        var fileTypes: [String] = []

    @Flag(name: .customLong("hidden"))     var hidden = false
    @Flag(name: .customLong("no-ignore"))  var noIgnore = false

    @Option(name: [.customShort("A"), .customLong("after-context")])  var afterContext = 0
    @Option(name: [.customShort("B"), .customLong("before-context")]) var beforeContext = 0
    @Option(name: [.customShort("C"), .customLong("context")])        var context = 0

    @Option(name: [.customShort("m"), .customLong("max-count")])      var maxCount: Int?
    @Option(name: .customLong("max-filesize"))                        var maxFilesize: String?

    @Flag(name: .customLong("json"))       var json = false   // selects output format

    func run() throws { /* unused; consumed by RipgrepKit */ }
    func toFFIRequest() -> SearchRequest { /* maps to UniFFI types */ }
}
```

**Notable defaults & FFI mapping (`toFFIRequest`):**
- `smart-case` is **on** by default (LLM-friendly; differs from `rg` CLI). `--no-smart-case` disables it via `inversion: .prefixedNo`.
- `respect_gitignore` ← `!noIgnore`.
- `context` (-C) sets both before/after when non-zero, unless either was set explicitly (mirror rg behavior).
- Empty `paths` → `["."]`.
- `glob` array splits by prefix: entries starting with `!` go into `exclude_globs` (with `!` stripped); the rest go into `include_globs`.
- `Options` (§6.1) mirrors `RipgrepArgs` field-for-field including the same defaults (notably `smartCase: true`, `respectGitignore: true`).

### 6.3 Tokenizer

`func tokenize(_ s: String) throws -> [String]` — minimal shell-style splitter:
- Splits on unquoted whitespace
- Honors single quotes (literal) and double quotes (with `\` escapes)
- Honors backslash escapes outside quotes
- **No** variable expansion, command substitution, or glob expansion

~30 LoC. Throws `Ripgrep.Error.invalidArguments` on unbalanced quotes.

### 6.4 Argument Parser Error Mapping

When `RipgrepArgs.parse(...)` throws, catch and convert via `RipgrepArgs.fullMessage(for:)` so the message includes usage hints. Wrap into `.invalidArguments(message:)`. The message string is what gets surfaced to the LLM as `ERROR: ...`, enabling self-correction.

## 7. LLM Tool Integration

```swift
extension Ripgrep {
    public struct ToolInput: Codable, Sendable {
        public let args: String
        public init(args: String) { self.args = args }
    }

    public static let toolSchema: String = #"""
    {
      "name": "ripgrep",
      "description": "Search files using ripgrep. Provide arguments exactly as you would on the rg CLI. Respects .gitignore by default. Examples:\n  \"TODO src/\"\n  \"'func\\s+\\w+' src/ -t swift -A 2\"\n  \"-i error logs/ -g '*.log' -m 50\"\nUse --json to get JSON-lines output for machine parsing.",
      "input_schema": {
        "type": "object",
        "required": ["args"],
        "properties": {
          "args": {
            "type": "string",
            "description": "rg-style argument string"
          }
        }
      }
    }
    """#

    public static func handleToolCall(_ input: ToolInput) async throws -> String {
        do {
            let result = try await run(input.args)
            return result.formattedAsText()  // or formattedAsJSONLines() if --json
        } catch let e as Ripgrep.Error {
            return "ERROR: \(e.message)"     // also returned as String, not thrown,
                                              // so LLM sees the failure and retries
        }
    }
}
```

**Output format selection:** `handleToolCall` checks whether `--json` was passed (by inspecting `RipgrepArgs.json` after parsing). Default is `formattedAsText()`. This requires `run(_:)` to expose the parsed `RipgrepArgs.json` value back to the caller — implementation will refactor `run` to return both `SearchResult` and the parsed args, or use a thread-local / context object. (Implementation plan will pick the cleanest path.)

**Typical consumer code (with any LLM SDK):**

```swift
case .toolUse(let block) where block.name == "ripgrep":
    let input = try block.input.decode(as: Ripgrep.ToolInput.self)
    let output = try await Ripgrep.handleToolCall(input)
    sendToolResult(output)
```

## 8. Build Process

### 8.1 First Build (developer)

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
                  aarch64-apple-darwin x86_64-apple-darwin
./scripts/build-xcframework.sh        # ~3-5 min on Apple Silicon
./scripts/generate-bindings.sh        # writes Sources/RipgrepKitFFI/ripgrep_core.swift
swift test
```

### 8.2 build-xcframework.sh

For each Rust target:
1. `cargo build --release --target <triple> -p ripgrep_core`
2. lipo simulator/macOS slices into fat binaries
3. Stage each into a temporary `.framework` dir with `Info.plist` + `module.modulemap`
4. `xcodebuild -create-xcframework -framework ... -framework ... -output Frameworks/RipgrepCore.xcframework`

### 8.3 generate-bindings.sh

```bash
cargo run --release --bin uniffi-bindgen --features cli -- \
    generate --library target/release/libripgrep_core.dylib \
    --language swift \
    --out-dir Sources/RipgrepKitFFI/
```

### 8.4 CI

`.github/workflows/release.yml` (macOS runner): on tag push, builds the xcframework, runs `swift test`, uploads `RipgrepCore.xcframework.zip` to the GitHub Release. Repo build remains usable without CI (xcframework checked in).

## 9. Testing Strategy

| Layer | Tool | What |
|---|---|---|
| Rust core | `cargo test` | Walker filtering, gitignore behavior, context lines, max_matches truncation, multiline, file types |
| Tokenizer | XCTest | Quotes, escapes, whitespace, malformed input |
| RipgrepArgs | XCTest | Each flag's short/long/inverted forms, conflicting flags, default propagation (smart-case on, paths→".") |
| End-to-end (Swift) | XCTest | `Ripgrep.run("...")` against `Tests/RipgrepKitTests/Fixtures/` mini-repo with .gitignore + hidden + multi-language files |
| Tool roundtrip | XCTest | JSON-encoded `ToolInput` → `handleToolCall` → assert output text/JSON shape |
| Error path | XCTest | Bad regex, missing path, malformed args; assert error messages contain usage hints |

Fixture repo (≈10 files): `.gitignore`, mix of `.swift`/`.rs`/`.ts`/`.txt`, one hidden file `.env`, one ignored file `target/foo.txt`, one large file (>1MB) for max-filesize tests.

## 10. Open Items Deferred to Implementation

- Exact crate version pinning (track ripgrep 15.1.0's `Cargo.lock`).
- How `handleToolCall` retrieves the `--json` flag from the parsed args (refactor `run` return type, vs. context object).
- Whether to vend a stable type-id for the tool (some SDKs require it).
- Whether to expose `formattedAsText` / `formattedAsJSONLines` as `RipgrepArgs.OutputFormat` enum.

## 11. Risk Notes

- **`grep-printer` not used**: We're rolling our own match collector to avoid `grep-printer`'s color/terminal coupling and to produce plain Swift structs. Lower risk than it sounds — the `Sink` trait is small and well-documented.
- **UniFFI async + Rust threading**: `WalkParallel` spawns its own threads internally; UniFFI's tokio runtime wraps the entry point. Need to confirm there's no nested-runtime issue (likely fine since `WalkParallel` uses `std::thread`, not tokio). Verify in implementation step 1.
- **XCFramework size**: Estimate ~5–10 MB per slice (Rust regex + ignore are not small). Acceptable for most apps; document in README.
- **swift-argument-parser exit behavior**: `parse()` does not exit the process (only `main()` does). Safe to use as a parsing-only tool inside a library.
