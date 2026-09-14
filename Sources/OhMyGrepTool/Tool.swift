import Foundation
import OhMyGrep

extension OhMyGrep {
    public struct ToolInput: Codable, Sendable {
        public let args: String
        public init(args: String) { self.args = args }
    }

    /// Anthropic Messages API tool schema.
    public static let toolSchema: String = #"""
    {
      "name": "oh_my_grep",
      "description": "Search files with oh-my-grep (ripgrep engine, rg-compatible subset). Provide arguments as you would on the rg CLI; behavior matches rg defaults (case-sensitive, respects .gitignore). Examples:\n  \"TODO src/\"\n  \"-S 'func\\s+\\w+' src/ -t swift -A 2\"\n  \"-i error logs/ -g '*.log' -m 50\"\nUnsupported flags: --pre, -z, --type-add, --hyperlink-format, --sort modified, --vimgrep, --binary. Use --json to get JSON-lines output (one match per line, no begin/end envelope).",
      "input_schema": {
        "type": "object",
        "required": ["args"],
        "properties": {
          "args": { "type": "string", "description": "rg-style argument string" }
        }
      }
    }
    """#

    public static func handleToolCall(_ input: ToolInput) async throws -> String {
        do {
            return try await run(input.args)
        } catch let e as OhMyGrep.Error {
            return "ERROR: \(e.message)"
        }
    }
}
