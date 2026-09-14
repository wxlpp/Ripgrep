# oh-my-grep

ripgrep's search engine for iOS and macOS apps. oh-my-grep wraps the Rust crates behind
[ripgrep](https://github.com/BurntSushi/ripgrep) (`ignore`, `grep-searcher`,
`grep-regex`) in a Swift package: a typed search API with bounded memory, a streaming
API, and an rg-style argument parser for LLM tool calling.

- iOS 16+, macOS 13+, Swift 6
- Respects `.gitignore` / `.ignore` / `.rgignore`, hidden files, globs and file types
- Parallel directory walk, cancellation and timeouts
- Memory stays bounded on large trees and huge lines (important on iOS, where exceeding
  the memory limit terminates the app)

## Installation

```swift
dependencies: [
    .package(url: "https://github.com/wxlpp/oh-my-grep", from: "0.1.0"),
],
targets: [
    .target(name: "MyApp", dependencies: [
        .product(name: "OhMyGrep", package: "oh-my-grep"),       // search API
        .product(name: "OhMyGrepTool", package: "oh-my-grep"),   // rg-style arguments / LLM tool
    ]),
]
```

Depend on a released version. `main` builds against a locally built XCFramework and is
meant for development (see [Building from source](#building-from-source)).

## Searching

```swift
import OhMyGrep

var options = OhMyGrep.Options()
options.fileTypes = ["swift"]
options.afterContext = 2
options.timeout = .seconds(5)

let result = try await OhMyGrep.search(pattern: #"func\s+\w+"#, in: [projectURL.path], options: options)
for match in result.matches {
    print(match.path, match.lineNumber, match.line, match.afterContext)
}
if result.truncated { print("stopped at \(options.maxMatches ?? 0) matches") }
for warning in result.warnings { print(warning.path, warning.message) }
```

`search` collects every match (up to `maxMatches`) and returns them sorted by path and
line. Paths must be given explicitly; there is no implicit current directory.

### Streaming

```swift
let summary = try await OhMyGrep.stream(pattern: "TODO", in: [root], options: options) { match in
    await model.append(match)   // awaited before more matches are fetched
}
```

`stream` delivers matches as they are found. The search waits while `onMatch` runs, so
a slow consumer slows the search instead of buffering results, and memory does not grow
with the number of matches. Matches from different files interleave; lines of one file
arrive in order. Cancelling the task stops the search and returns a summary with
`cancelled == true`; an error thrown by `onMatch` stops it and is rethrown.

### Options

| Option | Default | Notes |
|---|---|---|
| `caseInsensitive`, `smartCase`, `multiline` | `false` | Same meaning as rg `-i`, `-S`, `-U` |
| `include`, `exclude` | `[]` | Globs (rg `-g` / `-g '!…'`) |
| `fileTypes` | `[]` | rg type names, e.g. `swift`, `rust` |
| `respectGitignore` | `true` | `.gitignore`, `.ignore`, `.rgignore`, global git excludes |
| `requireGit` | `true` | Like rg, `.gitignore` applies only inside a git repository. **Set `false` in app sandboxes**, which rarely contain a `.git` directory |
| `includeHidden` | `false` | |
| `beforeContext`, `afterContext` | `0` | |
| `maxMatches` | `10_000` | Total limit; `nil` = unlimited. `truncated` means the limit was reached |
| `maxColumns` | `4096` | Longest returned line in bytes; `nil` = unlimited |
| `maxFiles`, `maxFileSizeBytes` | `nil` | |
| `timeout` | `nil` | The search stops and reports `cancelled` |
| `searchBinary` | `false` | Search files containing NUL bytes as text |

`Options` is `Codable`; missing keys decode to the defaults and an explicit `null`
limit means unlimited.

### Results

- **Context.** A match owns only the contiguous non-match lines next to it: before-context
  never reaches back past the previous match, and after-context ends at the next match.
  `formattedAsText()` renders rg-style `path:line:text` output with `--` separators;
  pass `contextSeparators: true` when you requested context (otherwise separators are
  inferred from whether any match carries context lines).
- **Submatches.** `Submatch.start`/`end` are byte offsets into the file's raw line. Use
  `match.submatchRanges` for `String.Index` ranges of `line`. Invalid UTF-8 is decoded
  lossily (U+FFFD), and ranges after the first replacement character are omitted.
- **Long lines.** A match line longer than `maxColumns` is cut to a window around its
  first submatch: `lineTruncated == true`, and `lineOffset` is the window's byte offset in
  the original line. Context lines are cut to a prefix.
- **Binary files.** Files reached by walking stop at the first NUL byte (a warning notes
  if lines matched before it); a file named directly reports `binary file matches`
  instead of returning lines — both as in rg.
- **Warnings.** Unreadable files, ignore-file syntax errors, binary files and oversized
  lines appear in `warnings` (at most 100, plus an `N more warnings omitted` note)
  instead of failing the search.

### Errors

`OhMyGrep.Error`: `.invalidArguments` (empty paths, bad globs or types, out-of-range
options), `.invalidPattern`, `.pathNotFound`, `.io`, `.internalPanic` (a bug in the
engine was caught instead of crashing the app).

## iOS notes

- **Paths.** Search inside your container (`FileManager.default.urls(for:in:)`) or
  folders the user picked. For security-scoped URLs from a document picker, call
  `startAccessingSecurityScopedResource()` before searching and stop afterwards.
- **Ignore files.** Set `requireGit = false` to honor `.gitignore` outside git checkouts.
- **Memory.** Each search keeps at most 16 MiB per worker thread for line buffers and a
  shared 128 MiB for whole-file buffers in multiline mode (larger files are skipped with
  a warning). `search` additionally holds up to `maxMatches` results; for unbounded
  result sets use `stream`. With `maxColumns: nil`, a stream can still hold up to 256
  very long matches in flight.
- **Privacy manifest.** The package ships `PrivacyInfo.xcprivacy` (File Timestamp API,
  reasons `C617.1` and `3B52.1`: file metadata of app-container and user-selected files).
- **Packaging.** The Rust core is a static library; nothing is embedded as a framework.
- **Threads.** A search uses up to 8 walker threads plus, for `stream`, one dispatch queue.

## LLM tool calling

```swift
import OhMyGrepTool

// Register OhMyGrep.toolSchema (Anthropic Messages API tool definition), then:
let output = await (try? OhMyGrep.handleToolCall(.init(args: toolArgs), workingDirectory: workspace.path))
```

`handleToolCall` parses rg-style arguments (`"-i error logs/ -g '*.log' -A 2"`), runs
the search, and returns text output; failures come back as `ERROR: …` strings. Relative
paths — and the implicit `.` when no path is given — resolve against `workingDirectory`;
without it, a relative path is an error. `OhMyGrep.run(_:workingDirectory:)` returns the
same output and throws instead.

| Flag | Meaning |
|---|---|
| `-i`, `-S`, `-U` | case-insensitive, smart case, multiline |
| `-g GLOB` | include glob; `-g '!GLOB'` excludes |
| `-t TYPE` | file type |
| `-a`, `--text` | search binary files as text |
| `--hidden`, `--no-ignore`, `--no-require-git` | hidden files, ignore files, `.gitignore` outside git |
| `-A N`, `-B N`, `-C N` | context; an explicit `-A 0`/`-B 0` overrides `-C` |
| `-m N` | total match limit (default 10 000) |
| `-M N`, `--max-columns N` | longest returned line in bytes (default 4096, `0` = unlimited) |
| `--max-files N`, `--max-filesize SIZE` | limits (`SIZE` like `5M`, `512K`) |
| `--timeout-ms N` | stop after N milliseconds |
| `--json` | JSON lines: matches, then `{"warning": …}` and `{"truncated": {"limit": N}}` |

Not supported: `--pre`, `-z`, `--type-add`, `--sort`, `--vimgrep`, replacement, PCRE2.

### Differences from rg

- `-m N` limits matches across the whole search, not per file.
- With `-m N` and after-context, rg keeps printing context past the Nth match; here the
  last match's after-context ends at the next match.
- rg prints warnings to stderr; here they follow the results in the same output.
- `-m 0` returns nothing (as in rg); the tool has no way to request unlimited matches —
  use the Swift API with `maxMatches: nil`.

## Security

Patterns use Rust's `regex` crate, which runs in linear time, so there is no
catastrophic backtracking. Untrusted input can still ask for a very large compiled
pattern or a huge directory tree: set `timeout` (or `--timeout-ms`) and the default
limits when patterns or paths come from a model or a user. The tool never runs a
shell; arguments are tokenized in-process, and symlinks are not followed.

## Building from source

Requirements: Xcode 16+, Rust (stable) with the Apple targets.

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin
bash scripts/generate-bindings.sh   # after changing the Rust API
bash scripts/build-xcframework.sh   # creates Frameworks/OhMyGrepCore.xcframework
cargo test -p ohmygrep_core
swift test
```

Fuzzing (nightly): `cd fuzz && cargo +nightly fuzz run search-request`.
Releases: see [docs/RELEASING.md](docs/RELEASING.md).

## License

MIT, as declared in `Cargo.toml`.
