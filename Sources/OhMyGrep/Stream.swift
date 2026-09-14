import Foundation
@preconcurrency import OhMyGrepFFI

extension OhMyGrep {
    /// The outcome of a streamed search, returned after the last match was delivered.
    public struct Summary: Codable, Sendable, Equatable {
        /// The match limit was reached (more matches may or may not exist).
        public var truncated: Bool
        public var cancelled: Bool
        public var filesSearched: Int
        /// Includes time the search waited for `onMatch`.
        public var elapsed: Duration
        public var warnings: [Warning]
    }

    /// Streams matches as they are found, so memory does not grow with the number of matches.
    ///
    /// `onMatch` is awaited before more matches are fetched: a slow consumer slows the
    /// search instead of buffering. Matches from different files interleave; lines within
    /// a file arrive in order. Cancelling the task stops the search and returns a summary
    /// with `cancelled == true`; an error thrown by `onMatch` stops it and is rethrown.
    @discardableResult
    public static func stream(
        pattern: String,
        in paths: [String],
        options: Options = .init(),
        onMatch: @Sendable (Match) async throws -> Void
    ) async throws -> Summary {
        let request = Unchecked(try options.toFFI(pattern: pattern, paths: paths))
        let token = Unchecked(CancelToken(timeoutMs: try options.timeout.map { try $0.ffiMilliseconds() }))
        let queue = DispatchQueue(label: "oh-my-grep.stream", qos: dispatchQoS(Task.currentPriority))

        return try await withTaskCancellationHandler {
            let session = try await onQueue(queue) {
                Unchecked(try SearchSession.start(request: request.value, cancel: token.value))
            }
            let fetch = { @Sendable () async throws -> Batch in
                try await onQueue(queue) { Batch(try session.value.nextBatch(max: 256)) }
            }
            while true {
                if Task.isCancelled {
                    // The handler may not have run yet; make sure Rust sees the cancel first.
                    token.value.cancel()
                }
                let batch = try await fetch()
                for match in batch.matches {
                    if Task.isCancelled { break }
                    do {
                        try await onMatch(match)
                    } catch {
                        token.value.cancel()
                        if batch.summary == nil { _ = try? await fetch() }
                        throw error
                    }
                }
                if var summary = batch.summary {
                    if Task.isCancelled { summary.cancelled = true }
                    return summary
                }
            }
        } onCancel: {
            token.value.cancel()
        }
    }
}

/// Wraps FFI values that are thread-safe on the Rust side but not marked `Sendable`
/// in the Swift 5 mode bindings.
private struct Unchecked<Value>: @unchecked Sendable {
    let value: Value
    init(_ value: Value) { self.value = value }
}

/// One FFI batch converted to Swift values on the worker queue.
private struct Batch: Sendable {
    let matches: [OhMyGrep.Match]
    let summary: OhMyGrep.Summary?

    init(_ ffi: SearchBatch) {
        matches = ffi.matches.map { OhMyGrep.Match(from: $0) }
        summary = ffi.summary.map {
            OhMyGrep.Summary(
                truncated: $0.truncated,
                cancelled: $0.cancelled,
                filesSearched: Int($0.filesSearched),
                elapsed: .milliseconds(Int($0.elapsedMs)),
                warnings: $0.warnings.map { OhMyGrep.Warning(path: $0.path, message: $0.message) }
            )
        }
    }
}

/// Runs blocking FFI work on `queue`, keeping it off the cooperative thread pool.
private func onQueue<T: Sendable>(
    _ queue: DispatchQueue,
    _ work: @escaping @Sendable () throws -> T
) async throws -> T {
    try await withCheckedThrowingContinuation { continuation in
        queue.async {
            do {
                continuation.resume(returning: try work())
            } catch let error as OhMyGrepError {
                continuation.resume(throwing: OhMyGrep.Error.from(error))
            } catch {
                continuation.resume(throwing: error)
            }
        }
    }
}

private func dispatchQoS(_ priority: TaskPriority) -> DispatchQoS {
    switch priority {
    case .high: return .userInitiated
    case .medium: return .default
    case .low: return .utility
    case .background: return .background
    default: return .default
    }
}
