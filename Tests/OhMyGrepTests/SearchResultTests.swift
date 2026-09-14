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
        XCTAssertEqual(r.formattedAsText(), "f.txt-9-before\nf.txt:10:match\nf.txt-11-after")
    }

    func testSeparatorInferredFromContext() {
        func match(_ line: Int, before: [String] = [], after: [String] = []) -> OhMyGrep.Match {
            OhMyGrep.Match(path: "f", lineNumber: line, line: "m", beforeContext: before,
                           afterContext: after, submatches: [])
        }
        let withContext = OhMyGrep.SearchResult(
            matches: [match(2, after: ["x"]), match(9, before: ["y"])],
            truncated: false, cancelled: false, filesSearched: 1, elapsed: .zero)
        XCTAssertEqual(withContext.formattedAsText(), "f:2:m\nf-3-x\n--\nf-8-y\nf:9:m")
        let plain = OhMyGrep.SearchResult(
            matches: [match(2), match(9)],
            truncated: false, cancelled: false, filesSearched: 1, elapsed: .zero)
        XCTAssertEqual(plain.formattedAsText(), "f:2:m\nf:9:m")
        XCTAssertEqual(plain.formattedAsText(contextSeparators: true), "f:2:m\n--\nf:9:m")
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
