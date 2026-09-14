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
        XCTAssertEqual(out, dir.appendingPathComponent("src/a.txt").standardizedFileURL.path + ":1:HIT src")
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
