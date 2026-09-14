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

    static func resolve(_ paths: [String], against workingDirectory: String?) throws(OhMyGrep.Error) -> [String] {
        try paths.map { (path: String) throws(OhMyGrep.Error) -> String in
            if path.hasPrefix("/") { return path }
            guard let base = workingDirectory else {
                throw .invalidArguments(
                    message: "relative path '\(path)' needs a workingDirectory")
            }
            return URL(fileURLWithPath: base, isDirectory: true)
                .appendingPathComponent(path).standardizedFileURL.path
        }
    }

    private static func runParsed(_ p: ParsedInvocation, workingDirectory: String?) async throws -> String {
        let paths = try resolve(p.paths, against: workingDirectory)
        let result = try await search(pattern: p.pattern, in: paths, options: p.options)
        switch p.outputFormat {
        case .text:
            let hasContext = p.options.beforeContext > 0 || p.options.afterContext > 0
            let notes = result.warnings.map { $0.path.isEmpty ? $0.message : "\($0.path): \($0.message)" }
            return ([result.formattedAsText(contextSeparators: hasContext)].filter { !$0.isEmpty } + notes)
                .joined(separator: "\n")
        case .jsonLines:
            return result.formattedAsJSONLines()
        }
    }
}
