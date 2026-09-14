import OhMyGrep

extension OhMyGrep {
    public static func run(_ argString: String) async throws -> String {
        try await runParsed(parse(argString))
    }
    public static func run(_ args: [String]) async throws -> String {
        try await runParsed(parse(args))
    }

    private static func runParsed(_ p: ParsedInvocation) async throws -> String {
        let result = try await search(pattern: p.pattern, in: p.paths, options: p.options)
        switch p.outputFormat {
        case .text:
            let hasContext = p.options.beforeContext > 0 || p.options.afterContext > 0
            return result.formattedAsText(contextSeparators: hasContext)
        case .jsonLines:
            return result.formattedAsJSONLines()
        }
    }
}
