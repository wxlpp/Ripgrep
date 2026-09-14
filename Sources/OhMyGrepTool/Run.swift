import Foundation
import OhMyGrep

extension OhMyGrep {
    /// Runs an rg-style argument string.
    ///
    /// - Parameter workingDirectory: base for relative paths, including the implicit
    ///   `.` when no path is given. Required whenever a path is relative: the process
    ///   working directory is meaningless in app sandboxes (it is `/` on iOS).
    public static func run(_ argString: String, workingDirectory: String? = nil) async throws -> String {
        try await runParsed(parse(argString), workingDirectory: workingDirectory)
    }

    public static func run(_ args: [String], workingDirectory: String? = nil) async throws -> String {
        try await runParsed(parse(args), workingDirectory: workingDirectory)
    }

    /// Joins relative paths onto `workingDirectory` textually, without resolving
    /// symlinks or `..`, so output paths keep the caller's prefix (e.g. `/private/var`).
    static func resolve(
        _ paths: [String], explicit: Bool, against workingDirectory: String?
    ) throws(OhMyGrep.Error) -> [String] {
        try paths.map { (path: String) throws(OhMyGrep.Error) -> String in
            if path.isEmpty { throw .invalidArguments(message: "empty path") }
            if path.hasPrefix("~") {
                throw .invalidArguments(message: "'~' is not expanded; use an absolute path: \(path)")
            }
            if path.hasPrefix("/") { return path }
            guard let base = workingDirectory else {
                throw .invalidArguments(message: explicit
                    ? "relative path '\(path)' needs a workingDirectory"
                    : "no path given; pass a workingDirectory or an absolute path")
            }
            let trimmed = base.hasSuffix("/") && base.count > 1 ? String(base.dropLast()) : base
            if path == "." { return trimmed }
            return trimmed == "/" ? "/\(path)" : "\(trimmed)/\(path)"
        }
    }

    private static func runParsed(_ p: ParsedInvocation, workingDirectory: String?) async throws -> String {
        let paths = try resolve(p.paths, explicit: p.pathsGiven, against: workingDirectory)
        let result = try await search(pattern: p.pattern, in: paths, options: p.options)
        switch p.outputFormat {
        case .text:
            let hasContext = p.options.beforeContext > 0 || p.options.afterContext > 0
            var notes = result.warnings.map { $0.path.isEmpty ? $0.message : "\($0.path): \($0.message)" }
            if result.truncated, let limit = p.options.maxMatches {
                notes.append("results truncated at \(limit) matches")
            }
            return ([result.formattedAsText(contextSeparators: hasContext)].filter { !$0.isEmpty } + notes)
                .joined(separator: "\n")
        case .jsonLines:
            var lines = result.formattedAsJSONLines()
            if result.truncated, let limit = p.options.maxMatches {
                lines += (lines.isEmpty ? "" : "\n") + #"{"truncated":{"limit":\#(limit)}}"#
            }
            return lines
        }
    }
}
