import XCTest
@testable import OhMyGrep

final class SearchResultTests: XCTestCase {
    func testFormattedAsTextSimple() {
        let r = OhMyGrep.SearchResult(
            matches: [
                OhMyGrep.Match(
                    path: "src/foo.swift", lineNumber: 10,
                    line: "// TODO: rename", beforeContext: [], afterContext: [],
                    submatches: []
                )
            ],
            truncated: false, cancelled: false,
            filesSearched: 1, elapsed: .milliseconds(2)
        )
        let out = r.formattedAsText()
        XCTAssertTrue(out.contains("src/foo.swift:10:// TODO: rename"))
    }

    func testFormattedAsTextWithContext() {
        let r = OhMyGrep.SearchResult(
            matches: [
                OhMyGrep.Match(
                    path: "f.txt", lineNumber: 10,
                    line: "match", beforeContext: ["before"], afterContext: ["after"],
                    submatches: []
                )
            ],
            truncated: false, cancelled: false,
            filesSearched: 1, elapsed: .milliseconds(0)
        )
        let out = r.formattedAsText()
        XCTAssertTrue(out.contains("f.txt-9-before"))
        XCTAssertTrue(out.contains("f.txt:10:match"))
        XCTAssertTrue(out.contains("f.txt-11-after"))
    }

    func testFormattedAsJSONLinesPerMatch() throws {
        let r = OhMyGrep.SearchResult(
            matches: [
                OhMyGrep.Match(
                    path: "a", lineNumber: 1, line: "x",
                    beforeContext: [], afterContext: [], submatches: []
                ),
                OhMyGrep.Match(
                    path: "b", lineNumber: 2, line: "y",
                    beforeContext: [], afterContext: [], submatches: []
                ),
            ],
            truncated: false, cancelled: false, filesSearched: 2, elapsed: .milliseconds(0)
        )
        let lines = r.formattedAsJSONLines().split(separator: "\n")
        XCTAssertEqual(lines.count, 2)  // no envelope, just per-match
    }
}
