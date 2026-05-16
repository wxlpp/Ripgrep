import Foundation
import ArgumentParser
import RipgrepKitCore

extension Ripgrep {
    public enum OutputFormat: String, Codable, Sendable { case text, jsonLines }

    public struct ParsedInvocation: Sendable {
        public let pattern: String
        public let paths: [String]
        public let options: Options
        public let outputFormat: OutputFormat
    }

    public static func parse(_ argString: String) throws -> ParsedInvocation {
        let tokens = try Tokenizer.tokenize(argString)
        return try parse(tokens)
    }

    public static func parse(_ args: [String]) throws -> ParsedInvocation {
        let parsed: RipgrepArgs
        do {
            parsed = try RipgrepArgs.parse(args)
        } catch {
            let msg = RipgrepArgs.fullMessage(for: error)
            throw Ripgrep.Error.invalidArguments(message: msg)
        }

        // Merge -C with -A/-B (rg semantics: -A/-B explicit override -C).
        let after = parsed.afterContext != 0 ? parsed.afterContext : parsed.context
        let before = parsed.beforeContext != 0 ? parsed.beforeContext : parsed.context

        // Split glob into include / exclude by `!` prefix.
        var include: [String] = []
        var exclude: [String] = []
        for g in parsed.glob {
            if g.hasPrefix("!") { exclude.append(String(g.dropFirst())) }
            else { include.append(g) }
        }

        // max-filesize (e.g. "5M", "1024K", "100"): absent → no limit;
        // present but unparseable (typo like "5X", "-5M") → error rather than
        // silently dropping the limit.
        let maxBytes: Int?
        if let rawMaxFilesize = parsed.maxFilesize {
            guard let bytes = parseFilesize(rawMaxFilesize) else {
                throw Ripgrep.Error.invalidArguments(
                    message: "invalid --max-filesize value: \(rawMaxFilesize)")
            }
            maxBytes = bytes
        } else {
            maxBytes = nil
        }

        let opts = Options(
            caseInsensitive: parsed.ignoreCase,
            smartCase: parsed.smartCase,
            multiline: parsed.multiline,
            include: include,
            exclude: exclude,
            fileTypes: parsed.fileTypes,
            respectGitignore: !parsed.noIgnore,
            includeHidden: parsed.hidden,
            beforeContext: before,
            afterContext: after,
            maxMatches: parsed.maxCount,
            maxFiles: parsed.maxFiles,
            maxFileSizeBytes: maxBytes,
            timeout: parsed.timeoutMs.map { .milliseconds($0) }
        )

        let paths = parsed.paths.isEmpty ? ["."] : parsed.paths
        return ParsedInvocation(
            pattern: parsed.pattern,
            paths: paths,
            options: opts,
            outputFormat: parsed.json ? .jsonLines : .text
        )
    }
}

private func parseFilesize(_ s: String) -> Int? {
    let s = s.trimmingCharacters(in: .whitespaces)
    guard !s.isEmpty else { return nil }
    let mult: Int
    let numericPart: String
    switch s.last! {
    case "K", "k": mult = 1024;            numericPart = String(s.dropLast())
    case "M", "m": mult = 1024*1024;       numericPart = String(s.dropLast())
    case "G", "g": mult = 1024*1024*1024;  numericPart = String(s.dropLast())
    default:       mult = 1;               numericPart = s
    }
    guard let n = Int(numericPart), n >= 0 else { return nil }
    return n * mult
}
