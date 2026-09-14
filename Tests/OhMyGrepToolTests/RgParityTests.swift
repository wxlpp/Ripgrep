import Foundation
import XCTest
@testable import OhMyGrepTool
@testable import OhMyGrep

/// Expected strings are `rg --no-heading -n -H` output (ripgrep 15.1) for the same files.
final class RgParityTests: XCTestCase {
    private var dir: URL!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("ohmygrep-parity-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        try Data("a\nb\nc\nd\nHIT one\ne\nHIT two\nf\n".utf8).write(to: file("ctx.txt"))
        try Data("l1\nHIT a\nl3\nl4\nl5\nl6\nHIT b\nl8\n".utf8).write(to: file("gap.txt"))
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: dir)
    }

    private func file(_ name: String) -> URL { dir.appendingPathComponent(name) }

    private func run(_ flags: [String], _ names: [String]) async throws -> String {
        try await OhMyGrep.run(flags + ["HIT"] + names.map { file($0).path })
    }

    /// Prefixes every non-separator line of an rg single-file listing with `path` + the rg delimiter.
    private func prefixed(_ name: String, _ body: String) -> String {
        let path = file(name).path
        return body.split(separator: "\n", omittingEmptySubsequences: false).map { line -> String in
            if line == "--" { return "--" }
            let delimiter = line.first(where: { $0 == ":" || $0 == "-" })!
            return "\(path)\(delimiter)\(line)"
        }.joined(separator: "\n")
    }

    func testBeforeContextStopsAtPreviousMatch() async throws {
        let out = try await run(["-B", "2"], ["ctx.txt"])
        XCTAssertEqual(out, prefixed("ctx.txt", "3-c\n4-d\n5:HIT one\n6-e\n7:HIT two"))
    }

    func testSymmetricContext() async throws {
        let out = try await run(["-C", "1"], ["ctx.txt"])
        XCTAssertEqual(out, prefixed("ctx.txt", "4-d\n5:HIT one\n6-e\n7:HIT two\n8-f"))
    }

    func testSeparatorBetweenDistantGroups() async throws {
        let out = try await run(["-C", "1"], ["gap.txt"])
        XCTAssertEqual(out, prefixed("gap.txt", "1-l1\n2:HIT a\n3-l3\n--\n6-l6\n7:HIT b\n8-l8"))
    }

    func testNoSeparatorWhenGroupsTouch() async throws {
        let out = try await run(["-C", "2"], ["gap.txt"])
        XCTAssertEqual(out, prefixed("gap.txt", "1-l1\n2:HIT a\n3-l3\n4-l4\n5-l5\n6-l6\n7:HIT b\n8-l8"))
    }

    func testSeparatorBetweenFiles() async throws {
        let out = try await run(["-C", "1"], ["ctx.txt", "gap.txt"])
        let expected = prefixed("ctx.txt", "4-d\n5:HIT one\n6-e\n7:HIT two\n8-f")
            + "\n--\n"
            + prefixed("gap.txt", "1-l1\n2:HIT a\n3-l3\n--\n6-l6\n7:HIT b\n8-l8")
        XCTAssertEqual(out, expected)
    }

    func testExplicitZeroAfterOverridesContext() async throws {
        let out = try await run(["-C", "3", "-A", "0"], ["gap.txt"])
        XCTAssertEqual(out, prefixed("gap.txt", "1-l1\n2:HIT a\n--\n4-l4\n5-l5\n6-l6\n7:HIT b"))
    }

    func testNoSeparatorsWithoutContext() async throws {
        let out = try await run([], ["ctx.txt", "gap.txt"])
        let expected = prefixed("ctx.txt", "5:HIT one\n7:HIT two") + "\n"
            + prefixed("gap.txt", "2:HIT a\n7:HIT b")
        XCTAssertEqual(out, expected)
    }

    func testMultilineMatchRendersEachLine() async throws {
        try Data("a\nHIT1\nmid\nHIT2\nb\nc\nd\ne\n".utf8).write(to: file("ml.txt"))
        let out = try await OhMyGrep.run(["-U", "-B", "2", "-A", "2", #"HIT1\nmid\nHIT2"#, file("ml.txt").path])
        XCTAssertEqual(out, prefixed("ml.txt", "1-a\n2:HIT1\n3:mid\n4:HIT2\n5-b\n6-c"))
    }

    func testMultilineTrailingBlankLine() async throws {
        try Data("x\nfoo\n\nbar\n".utf8).write(to: file("blank.txt"))
        let out = try await OhMyGrep.run(["-U", #"foo\n\n"#, file("blank.txt").path])
        XCTAssertEqual(out, prefixed("blank.txt", "2:foo\n3:"))
    }

    func testBinaryFilesSkippedUnlessText() async throws {
        try Data("x\u{0}y HIT\n".utf8).write(to: file("blob.dat"))
        let skipped = try await OhMyGrep.run(["HIT", dir.path])
        XCTAssertFalse(skipped.contains("blob.dat"), skipped)
        let included = try await OhMyGrep.run(["-a", "HIT", dir.path])
        XCTAssertTrue(included.contains("blob.dat:1:"), included)
    }
}
