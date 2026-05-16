import RipgrepKitCore

extension Ripgrep {
    public static func run(_ argString: String) async throws -> String {
        try await runParsed(parse(argString))
    }
    public static func run(_ args: [String]) async throws -> String {
        try await runParsed(parse(args))
    }

    private static func runParsed(_ p: ParsedInvocation) async throws -> String {
        let result = try await search(pattern: p.pattern, in: p.paths, options: p.options)
        switch p.outputFormat {
        case .text:      return result.formattedAsText()
        case .jsonLines: return result.formattedAsJSONLines()
        }
    }
}
