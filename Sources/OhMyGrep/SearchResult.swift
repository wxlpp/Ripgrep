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

    public struct SearchResult: Codable, Sendable {
        public let matches: [Match]
        public let truncated: Bool
        public let cancelled: Bool
        public let filesSearched: Int
        public let elapsed: Duration

        public init(matches: [Match], truncated: Bool, cancelled: Bool,
                    filesSearched: Int, elapsed: Duration) {
            self.matches = matches; self.truncated = truncated
            self.cancelled = cancelled; self.filesSearched = filesSearched
            self.elapsed = elapsed
        }

        /// Renders like `rg --no-heading -n`: `path:line:text` for matches,
        /// `path-line-text` for context, and `--` between non-adjacent groups.
        ///
        /// - Parameter contextSeparators: emit `--` separators, as rg does when
        ///   `-A`/`-B`/`-C` is set. `nil` infers it from whether any match has context.
        public func formattedAsText(contextSeparators: Bool? = nil) -> String {
            let separate = contextSeparators
                ?? matches.contains { !$0.beforeContext.isEmpty || !$0.afterContext.isEmpty }
            var lines: [String] = []
            var last: (path: String, line: Int)? = nil
            for m in matches {
                let firstLine = m.lineNumber - m.beforeContext.count
                if separate, let last, last.path != m.path || firstLine > last.line + 1 {
                    lines.append("--")
                }
                last = (m.path, m.lineNumber + m.afterContext.count)
                let baseLine = m.lineNumber
                for (i, b) in m.beforeContext.enumerated() {
                    let ln = baseLine - (m.beforeContext.count - i)
                    lines.append("\(m.path)-\(ln)-\(b)")
                }
                lines.append("\(m.path):\(baseLine):\(m.line)")
                for (i, a) in m.afterContext.enumerated() {
                    let ln = baseLine + i + 1
                    lines.append("\(m.path)-\(ln)-\(a)")
                }
            }
            return lines.joined(separator: "\n")
        }

        /// - Note: `try?` on `enc.encode` is safe because every `Match` field is
        ///   trivially `Encodable` (String/Int/[String]/[Submatch]); a match can
        ///   never be silently dropped today. If a future non-trivially-Encodable
        ///   field is added to `Match`, replace this with explicit error handling.
        public func formattedAsJSONLines() -> String {
            let enc = JSONEncoder()
            enc.outputFormatting = []
            return matches.compactMap { m in
                guard let data = try? enc.encode(m),
                      let s = String(data: data, encoding: .utf8) else { return nil }
                return s
            }.joined(separator: "\n")
        }
    }
}
