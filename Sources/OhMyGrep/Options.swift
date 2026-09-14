import Foundation
@preconcurrency import OhMyGrepFFI

extension Duration {
    /// Whole milliseconds (floored), suitable for FFI `UInt64?` timeout fields.
    ///
    /// Throws `OhMyGrep.Error.invalidArguments` for negative or overflow-producing
    /// durations so that a public `Codable` `Options.timeout` never causes an
    /// uncatchable host-process trap (same invariant as the `UInt32` range guards
    /// in `toFFI`).
    func ffiMilliseconds() throws(OhMyGrep.Error) -> UInt64 {
        guard self >= .zero else {
            throw .invalidArguments(
                message: "timeout must be non-negative, got \(self)")
        }
        let c = components
        // Check `seconds * 1000` for Int64 overflow.
        let (sMs, ov) = c.seconds.multipliedReportingOverflow(by: 1000)
        guard !ov else {
            throw .invalidArguments(
                message: "timeout too large to represent in milliseconds, got \(self)")
        }
        // attoseconds is in [0, 999_999_999_999_999_999] for a normalised
        // non-negative Duration, so this division is always non-negative and
        // fits Int64; no overflow possible.
        let attoMs = c.attoseconds / 1_000_000_000_000_000
        let totalMs = sMs + attoMs
        // totalMs >= 0 because both terms are non-negative for a non-negative Duration.
        guard let result = UInt64(exactly: totalMs) else {
            throw .invalidArguments(
                message: "timeout too large to represent in milliseconds, got \(self)")
        }
        return result
    }
}

extension OhMyGrep {
    /// - Note: `Codable` uses synthesized coding keys. A future **non-optional**
    ///   field would break decoding of v0.1.0-encoded JSON (keyNotFound). Add
    ///   such fields as optional, or provide a custom `init(from:)` using
    ///   `decodeIfPresent` with a default, to preserve backward compatibility.
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

extension OhMyGrep.Options {
    // Throws (not `precondition`) on out-of-range fields: `Options` is a public
    // `Codable` value, so a JSON-decoded / hand-constructed instance with a
    // negative or overflowing field must be a recoverable error, not a
    // host-process trap (UInt32.init traps on overflow for 64-bit Int values).
    func toFFI(pattern: String, paths: [String]) throws(OhMyGrep.Error) -> SearchRequest {
        guard (0...Int(UInt32.max)).contains(beforeContext) else {
            throw .invalidArguments(
                message: "beforeContext out of range [0, \(UInt32.max)], got \(beforeContext)")
        }
        guard (0...Int(UInt32.max)).contains(afterContext) else {
            throw .invalidArguments(
                message: "afterContext out of range [0, \(UInt32.max)], got \(afterContext)")
        }

        let ffiMaxMatches: UInt32?
        if let m = maxMatches {
            guard (0...Int(UInt32.max)).contains(m) else {
                throw .invalidArguments(
                    message: "maxMatches out of range [0, \(UInt32.max)], got \(m)")
            }
            ffiMaxMatches = UInt32(m)
        } else { ffiMaxMatches = nil }

        let ffiMaxFiles: UInt32?
        if let f = maxFiles {
            guard (0...Int(UInt32.max)).contains(f) else {
                throw .invalidArguments(
                    message: "maxFiles out of range [0, \(UInt32.max)], got \(f)")
            }
            ffiMaxFiles = UInt32(f)
        } else { ffiMaxFiles = nil }

        // Int.max < UInt64.max on 64-bit, so only the lower bound can fail here.
        let ffiMaxFileSizeBytes: UInt64?
        if let s = maxFileSizeBytes {
            guard s >= 0 else {
                throw .invalidArguments(
                    message: "maxFileSizeBytes must be ≥ 0, got \(s)")
            }
            ffiMaxFileSizeBytes = UInt64(s)
        } else { ffiMaxFileSizeBytes = nil }

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
            maxMatches: ffiMaxMatches,
            maxFiles: ffiMaxFiles,
            maxFileSizeBytes: ffiMaxFileSizeBytes
        )
    }
}
