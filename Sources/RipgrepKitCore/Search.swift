import Foundation
@preconcurrency import RipgrepKitFFI

// CancelToken is a Swift-5-mode class with no Sendable conformance.
// The underlying Rust CancelToken uses an atomic flag and is explicitly
// designed for cross-thread cancellation, so @unchecked Sendable is safe.
private struct CancelHandle: @unchecked Sendable {
    let token: CancelToken
}

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
        let handle = CancelHandle(token: CancelToken(timeoutMs: timeoutMs))
        return try await withTaskCancellationHandler {
            try await Task.detached(priority: .userInitiated) {
                do {
                    let ffi = try searchBlocking(request: request, cancel: handle.token)
                    return SearchResult(from: ffi)
                } catch let e as RipgrepError {
                    throw Ripgrep.Error.from(e)
                }
            }.value
        } onCancel: {
            handle.token.cancel()
        }
    }
}

extension Ripgrep.SearchResult {
    init(from ffi: RipgrepKitFFI.SearchResult) {
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
    init(from ffi: RipgrepKitFFI.SearchMatch) {
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
