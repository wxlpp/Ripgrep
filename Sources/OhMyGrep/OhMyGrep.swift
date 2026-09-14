import OhMyGrepFFI

/// Namespace for the OhMyGrep API.
///
/// `OhMyGrep` is an intentionally empty (no-case) enum used purely as a
/// namespace; its surface is composed across files via extensions
/// (`OhMyGrep.Error`, `.Options`, `.SearchResult`, `.search`, and — in
/// `OhMyGrepTool` — `.parse`, `.run`, `.handleToolCall`, `.toolSchema`).
public enum OhMyGrep {}
