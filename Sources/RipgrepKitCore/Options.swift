import Foundation
@preconcurrency import RipgrepKitFFI

extension Duration {
    /// Whole milliseconds (floored), suitable for FFI `UInt64?` timeout fields.
    var ffiMilliseconds: UInt64 {
        let c = components
        return UInt64(c.seconds * 1000 + c.attoseconds / 1_000_000_000_000_000)
    }
}

extension Ripgrep {
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

extension Ripgrep.Options {
    func toFFI(pattern: String, paths: [String]) -> SearchRequest {
        precondition(beforeContext >= 0, "beforeContext must be ≥ 0")
        precondition(afterContext >= 0, "afterContext must be ≥ 0")
        precondition((maxMatches ?? 0) >= 0, "maxMatches must be ≥ 0")
        precondition((maxFiles ?? 0) >= 0, "maxFiles must be ≥ 0")
        precondition((maxFileSizeBytes ?? 0) >= 0, "maxFileSizeBytes must be ≥ 0")

        let timeoutMs: UInt64? = timeout?.ffiMilliseconds

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
