import RipgrepKitFFI

/// Namespace for the RipgrepKit API.
///
/// `Ripgrep` is an intentionally empty (no-case) enum used purely as a
/// namespace; its surface is composed across files via extensions
/// (`Ripgrep.Error`, `.Options`, `.SearchResult`, `.search`, and — in
/// `RipgrepKitTool` — `.parse`, `.run`, `.handleToolCall`, `.toolSchema`).
public enum Ripgrep {}
