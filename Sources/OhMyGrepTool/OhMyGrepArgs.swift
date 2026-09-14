import ArgumentParser

struct OhMyGrepArgs: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "rg",
        abstract: "Search files using ripgrep semantics (subset)."
    )

    @Argument var pattern: String
    @Argument var paths: [String] = []

    @Flag(name: [.short, .customLong("ignore-case")])
    var ignoreCase: Bool = false

    @Flag(name: [.customShort("S"), .customLong("smart-case")])
    var smartCase: Bool = false

    @Flag(name: [.customShort("U"), .customLong("multiline")])
    var multiline: Bool = false

    @Option(name: [.customShort("g"), .customLong("glob")])
    var glob: [String] = []

    @Option(name: [.customShort("t"), .customLong("type")])
    var fileTypes: [String] = []

    @Flag(name: .customLong("hidden"))
    var hidden: Bool = false

    @Flag(name: .customLong("no-ignore"))
    var noIgnore: Bool = false

    @Option(name: [.customShort("A"), .customLong("after-context")])
    var afterContext: Int = 0

    @Option(name: [.customShort("B"), .customLong("before-context")])
    var beforeContext: Int = 0

    @Option(name: [.customShort("C"), .customLong("context")])
    var context: Int = 0

    @Option(name: [.customShort("m"), .customLong("max-count")])
    var maxCount: Int?

    @Option(name: .customLong("max-files"))
    var maxFiles: Int?

    @Option(name: .customLong("max-filesize"))
    var maxFilesize: String?

    @Option(name: .customLong("timeout-ms"))
    var timeoutMs: Int?

    @Flag(name: .customLong("json"))
    var json: Bool = false

    func run() throws { /* unused; see Parse.swift */ }
}
