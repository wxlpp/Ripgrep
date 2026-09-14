import Foundation
import XCTest
@testable import OhMyGrepTool
@testable import OhMyGrep

final class WorkingDirectoryTests: XCTestCase {
    private var dir: URL!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("ohmygrep-wd-\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: dir.appendingPathComponent("src"), withIntermediateDirectories: true)
        try Data("HIT src\n".utf8).write(to: dir.appendingPathComponent("src/a.txt"))
        try Data("HIT root\n".utf8).write(to: dir.appendingPathComponent("b.txt"))
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: dir)
    }

    func testRelativePathResolvesAgainstWorkingDirectory() async throws {
        let out = try await OhMyGrep.run("HIT src", workingDirectory: dir.path)
        XCTAssertEqual(out, dir.path + "/src/a.txt:1:HIT src")
    }

    func testResolutionIsTextual() throws {
        XCTAssertEqual(try OhMyGrep.resolve(["x", "."], explicit: true, against: "/private/tmp/"),
                       ["/private/tmp/x", "/private/tmp"])
        XCTAssertEqual(try OhMyGrep.resolve(["a/../b"], explicit: true, against: "/private/var"),
                       ["/private/var/a/../b"])
        XCTAssertEqual(try OhMyGrep.resolve(["x"], explicit: true, against: "/"), ["/x"])
    }

    func testTildeAndEmptyPathsAreRejected() {
        for path in ["~/x", ""] {
            XCTAssertThrowsError(try OhMyGrep.resolve([path], explicit: true, against: "/tmp"))
        }
    }

    func testImplicitPathWithoutWorkingDirectoryExplainsItself() async throws {
        let out = try await OhMyGrep.handleToolCall(.init(args: "HIT"))
        XCTAssertTrue(out.contains("no path given"), out)
    }

    func testRustInvalidArgumentsMapsWithoutPrefix() async throws {
        do {
            _ = try await OhMyGrep.run(["-g", "a{", "HIT", dir.path])
            XCTFail("expected throw")
        } catch let e as OhMyGrep.Error {
            guard case .invalidArguments(let message) = e else { return XCTFail("wrong error: \(e)") }
            XCTAssertTrue(message.hasPrefix("glob:"), message)
        }
    }

    func testNoPathSearchesWorkingDirectory() async throws {
        let out = try await OhMyGrep.run("HIT", workingDirectory: dir.path)
        XCTAssertEqual(out.split(separator: "\n").count, 2, out)
    }

    func testRelativePathWithoutWorkingDirectoryIsAnError() async throws {
        let out = try await OhMyGrep.handleToolCall(.init(args: "HIT src"))
        XCTAssertTrue(out.hasPrefix("ERROR:"), out)
        XCTAssertTrue(out.contains("workingDirectory"), out)
    }

    func testAbsolutePathIgnoresWorkingDirectory() async throws {
        let out = try await OhMyGrep.handleToolCall(
            .init(args: "HIT \(dir.appendingPathComponent("b.txt").path)"), workingDirectory: "/nonexistent")
        XCTAssertTrue(out.hasSuffix(":1:HIT root"), out)
    }

    func testWarningsAppearInTextAndJSONOutput() async throws {
        let blob = dir.appendingPathComponent("blob.dat")
        try Data("x\u{0}y HIT\n".utf8).write(to: blob)
        let text = try await OhMyGrep.run(["HIT", blob.path])
        XCTAssertTrue(text.hasPrefix("\(blob.path): binary file matches"), text)

        let json = try await OhMyGrep.run(["--json", "HIT", blob.path])
        let object = try JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any]
        let warning = object?["warning"] as? [String: String]
        XCTAssertEqual(warning?["path"], blob.path)
    }
}
