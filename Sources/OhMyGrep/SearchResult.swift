import Foundation

extension OhMyGrep {
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

    /// A per-path problem that did not stop the search. `path` is empty for summary notes.
    public struct Warning: Codable, Sendable, Equatable {
        public let path: String
        public let message: String
        public init(path: String, message: String) { self.path = path; self.message = message }
    }

    public struct SearchResult: Codable, Sendable {
        public let matches: [Match]
        public let truncated: Bool
        public let cancelled: Bool
        public let filesSearched: Int
        public let elapsed: Duration
        /// Unreadable paths, binary files and similar; capped at 100 plus an overflow note.
        public let warnings: [Warning]

        public init(matches: [Match], truncated: Bool, cancelled: Bool,
                    filesSearched: Int, elapsed: Duration, warnings: [Warning] = []) {
            self.matches = matches; self.truncated = truncated
            self.cancelled = cancelled; self.filesSearched = filesSearched
            self.elapsed = elapsed
            self.warnings = warnings
        }

        private enum CodingKeys: String, CodingKey {
            case matches, truncated, cancelled, filesSearched, elapsed, warnings
        }

        public init(from decoder: any Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            matches = try c.decode([Match].self, forKey: .matches)
            truncated = try c.decode(Bool.self, forKey: .truncated)
            cancelled = try c.decode(Bool.self, forKey: .cancelled)
            filesSearched = try c.decode(Int.self, forKey: .filesSearched)
            elapsed = try c.decode(Duration.self, forKey: .elapsed)
            warnings = try c.decodeIfPresent([Warning].self, forKey: .warnings) ?? []
        }

        /// Renders like `rg --no-heading -n -H`: `path:line:text` for each matched
        /// line (multiline matches get one row per line), `path-line-text` for
        /// context, and `--` between non-adjacent groups.
        ///
        /// - Parameter contextSeparators: emit `--` separators, as rg does when
        ///   `-A`/`-B`/`-C` is set. `nil` infers it from whether any match has context.
        public func formattedAsText(contextSeparators: Bool? = nil) -> String {
            let separate = contextSeparators
                ?? matches.contains { !$0.beforeContext.isEmpty || !$0.afterContext.isEmpty }
            var lines: [String] = []
            var last: (path: String, line: Int)? = nil
            for m in matches {
                // Split on scalars: Swift treats "\r\n" as one Character, not "\n".
                let matchLines = m.line.unicodeScalars
                    .split(separator: "\n", omittingEmptySubsequences: false)
                    .map { row -> String in
                        var row = String.UnicodeScalarView(row)
                        if row.last == "\r" { row.removeLast() }
                        return String(row)
                    }
                let firstLine = m.lineNumber - m.beforeContext.count
                let lastMatchLine = m.lineNumber + matchLines.count - 1
                if separate, let last, last.path != m.path || firstLine > last.line + 1 {
                    lines.append("--")
                }
                last = (m.path, lastMatchLine + m.afterContext.count)
                for (i, b) in m.beforeContext.enumerated() {
                    lines.append("\(m.path)-\(firstLine + i)-\(b)")
                }
                for (i, text) in matchLines.enumerated() {
                    lines.append("\(m.path):\(m.lineNumber + i):\(text)")
                }
                for (i, a) in m.afterContext.enumerated() {
                    lines.append("\(m.path)-\(lastMatchLine + i + 1)-\(a)")
                }
            }
            return lines.joined(separator: "\n")
        }

        /// One JSON object per match, then one `{"warning": {...}}` object per warning.
        public func formattedAsJSONLines() -> String {
            let enc = JSONEncoder()
            enc.outputFormatting = [.sortedKeys]
            // Encoding plain String/Int/array fields cannot fail.
            func json(_ value: some Encodable) -> String {
                String(decoding: try! enc.encode(value), as: UTF8.self)
            }
            return (matches.map(json) + warnings.map { json(["warning": $0]) })
                .joined(separator: "\n")
        }
    }
}
