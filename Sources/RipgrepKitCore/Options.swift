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
