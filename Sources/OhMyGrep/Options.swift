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
    /// Search options. Decoding tolerates missing keys (they take the `init()`
    /// defaults); an explicit `null` for an optional limit means "no limit".
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
        /// Search files containing NUL bytes as text; by default they are skipped.
        public var searchBinary: Bool

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
            timeout: Duration? = nil,
            searchBinary: Bool = false
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
            self.searchBinary = searchBinary
        }

        private enum CodingKeys: String, CodingKey {
            case caseInsensitive, smartCase, multiline, include, exclude, fileTypes
            case respectGitignore, includeHidden, beforeContext, afterContext
            case maxMatches, maxFiles, maxFileSizeBytes, timeout, searchBinary
        }

        public init(from decoder: any Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            let d = Options()
            func value<T: Decodable>(_ key: CodingKeys, _ fallback: T) throws -> T {
                try c.decodeIfPresent(T.self, forKey: key) ?? fallback
            }
            func optional<T: Decodable>(_ key: CodingKeys, _ fallback: T?) throws -> T? {
                c.contains(key) ? try c.decodeIfPresent(T.self, forKey: key) : fallback
            }
            caseInsensitive = try value(.caseInsensitive, d.caseInsensitive)
            smartCase = try value(.smartCase, d.smartCase)
            multiline = try value(.multiline, d.multiline)
            include = try value(.include, d.include)
            exclude = try value(.exclude, d.exclude)
            fileTypes = try value(.fileTypes, d.fileTypes)
            respectGitignore = try value(.respectGitignore, d.respectGitignore)
            includeHidden = try value(.includeHidden, d.includeHidden)
            beforeContext = try value(.beforeContext, d.beforeContext)
            afterContext = try value(.afterContext, d.afterContext)
            maxMatches = try optional(.maxMatches, d.maxMatches)
            maxFiles = try optional(.maxFiles, d.maxFiles)
            maxFileSizeBytes = try optional(.maxFileSizeBytes, d.maxFileSizeBytes)
            timeout = try optional(.timeout, d.timeout)
            searchBinary = try value(.searchBinary, d.searchBinary)
        }

        public func encode(to encoder: any Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(caseInsensitive, forKey: .caseInsensitive)
            try c.encode(smartCase, forKey: .smartCase)
            try c.encode(multiline, forKey: .multiline)
            try c.encode(include, forKey: .include)
            try c.encode(exclude, forKey: .exclude)
            try c.encode(fileTypes, forKey: .fileTypes)
            try c.encode(respectGitignore, forKey: .respectGitignore)
            try c.encode(includeHidden, forKey: .includeHidden)
            try c.encode(beforeContext, forKey: .beforeContext)
            try c.encode(afterContext, forKey: .afterContext)
            // Optional limits encode `null` explicitly so nil survives a round trip.
            try c.encode(maxMatches, forKey: .maxMatches)
            try c.encode(maxFiles, forKey: .maxFiles)
            try c.encode(maxFileSizeBytes, forKey: .maxFileSizeBytes)
            try c.encode(timeout, forKey: .timeout)
            try c.encode(searchBinary, forKey: .searchBinary)
        }
    }
}

extension OhMyGrep.Options {
    // Throws rather than trapping: decoded or hand-built values can be out of range.
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
            maxFileSizeBytes: ffiMaxFileSizeBytes,
            searchBinary: searchBinary
        )
    }
}
