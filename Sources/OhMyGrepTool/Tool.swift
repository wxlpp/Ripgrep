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
      "description": "Search files with oh-my-grep (ripgrep engine, rg-compatible subset). Pass arguments as on the rg command line.\nSupported: -i, -S, -U, -g GLOB (prefix ! to exclude), -t TYPE, -a/--text, --hidden, --no-ignore, --no-require-git, -A/-B/-C N, -m N (total match limit, default 10000), -M N (max line bytes, default 4096, 0 = unlimited), --max-files N, --max-filesize SIZE, --timeout-ms N, --json.\nNot supported: --pre, -z, --type-add, --sort, --vimgrep, replacement, PCRE2.\nRelative paths, and the default path when none is given, resolve against the host-provided working directory. Binary files are skipped (a named binary file reports \"binary file matches\"). Lines longer than -M are cut to a window around the first match.\nOutput: rg-style `path:line:text` lines, then warnings as `path: message` lines, then `results truncated at N matches` if the limit was hit. --json prints one match object per line, then {\"warning\": {...}} objects and {\"truncated\": {\"limit\": N}}.\nExamples:\n  \"TODO src/\"\n  \"-S 'func\\s+\\w+' src/ -t swift -A 2\"\n  \"-i error logs/ -g '*.log' -m 50\"",
      "input_schema": {
        "type": "object",
        "required": ["args"],
        "properties": {
          "args": { "type": "string", "description": "rg-style argument string" }
        }
      }
    }
    """#

    /// - Parameter workingDirectory: base for relative paths in the model's arguments.
    public static func handleToolCall(_ input: ToolInput, workingDirectory: String? = nil) async throws -> String {
        do {
            return try await run(input.args, workingDirectory: workingDirectory)
        } catch let e as OhMyGrep.Error {
            return "ERROR: \(e.message)"
        }
    }
}
