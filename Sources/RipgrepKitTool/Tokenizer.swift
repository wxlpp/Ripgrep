import RipgrepKitCore

public enum Tokenizer {
    public static func tokenize(_ input: String) throws(Ripgrep.Error) -> [String] {
        var tokens: [String] = []
        var current = ""
        var inSingle = false
        var inDouble = false
        var escaped = false
        var hasContent = false

        for ch in input {
            if escaped {
                current.append(ch)
                escaped = false
                hasContent = true
                continue
            }
            if ch == "\\" && !inSingle {
                escaped = true
                continue
            }
            if ch == "'" && !inDouble { inSingle.toggle(); hasContent = true; continue }
            if ch == "\"" && !inSingle { inDouble.toggle(); hasContent = true; continue }

            if !inSingle && !inDouble && ch.isWhitespace {
                if hasContent {
                    tokens.append(current)
                    current = ""
                    hasContent = false
                }
                continue
            }
            current.append(ch)
            hasContent = true
        }

        if inSingle || inDouble {
            throw .invalidArguments(message: "unbalanced quote in argument string")
        }
        if escaped {
            throw .invalidArguments(message: "dangling backslash escape")
        }
        if hasContent { tokens.append(current) }
        return tokens
    }
}
