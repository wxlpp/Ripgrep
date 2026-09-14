import Foundation
import XCTest
@testable import OhMyGrep

final class SubmatchRangesTests: XCTestCase {
    private func search(_ bytes: [UInt8], _ pattern: String) async throws -> OhMyGrep.Match {
        let file = FileManager.default.temporaryDirectory
            .appendingPathComponent("ohmygrep-ranges-\(UUID().uuidString).txt")
        try Data(bytes).write(to: file)
        defer { try? FileManager.default.removeItem(at: file) }
        let result = try await OhMyGrep.search(pattern: pattern, in: [file.path])
        return try XCTUnwrap(result.matches.first)
    }

    func testRangesSelectMatchedTextAfterMultibyteCharacters() async throws {
        let m = try await search(Array("héllo — TODO and TODO\n".utf8), "TODO")
        XCTAssertEqual(m.submatchRanges.map { String(m.line[$0]) }, ["TODO", "TODO"])
    }

    func testRangesAfterInvalidUTF8AreOmitted() async throws {
        let before = try await search(Array("ab TODO ".utf8) + [0xFF] + Array("\n".utf8), "TODO")
        XCTAssertEqual(before.submatchRanges.map { String(before.line[$0]) }, ["TODO"])

        let after = try await search([0xFF] + Array("ab TODO\n".utf8), "TODO")
        XCTAssertEqual(after.submatches.count, 1)
        XCTAssertTrue(after.submatchRanges.isEmpty)
    }

    func testOutOfRangeOrMidScalarOffsetsAreOmitted() {
        let m = OhMyGrep.Match(path: "p", lineNumber: 1, line: "é", beforeContext: [], afterContext: [],
                               submatches: [.init(start: 1, end: 2), .init(start: 0, end: 5), .init(start: 0, end: 2)])
        XCTAssertEqual(m.submatchRanges.map { String(m.line[$0]) }, ["é"])
    }
}
