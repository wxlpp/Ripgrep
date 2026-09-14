import XCTest
@testable import OhMyGrep

final class OptionsTests: XCTestCase {
    func testDefaultsMatchRipgrepCLI() {
        let o = OhMyGrep.Options()
        XCTAssertFalse(o.caseInsensitive)
        XCTAssertFalse(o.smartCase)        // matches rg CLI default
        XCTAssertFalse(o.multiline)
        XCTAssertTrue(o.respectGitignore)
        XCTAssertFalse(o.includeHidden)
        XCTAssertEqual(o.beforeContext, 0)
        XCTAssertEqual(o.afterContext, 0)
        XCTAssertNil(o.maxMatches)
        XCTAssertNil(o.timeout)
        XCTAssertTrue(o.include.isEmpty)
        XCTAssertTrue(o.exclude.isEmpty)
        XCTAssertTrue(o.fileTypes.isEmpty)
        XCTAssertNil(o.maxFiles)
        XCTAssertNil(o.maxFileSizeBytes)
    }

    func testCodableRoundtrip() throws {
        var o = OhMyGrep.Options()
        o.beforeContext = 2
        o.include = ["*.swift"]
        o.timeout = .milliseconds(500)
        let data = try JSONEncoder().encode(o)
        let decoded = try JSONDecoder().decode(OhMyGrep.Options.self, from: data)
        XCTAssertEqual(decoded.beforeContext, 2)
        XCTAssertEqual(decoded.include, ["*.swift"])
        XCTAssertEqual(decoded.timeout, .milliseconds(500))
        XCTAssertNil(decoded.maxMatches)
        XCTAssertNil(decoded.maxFiles)
        XCTAssertNil(decoded.maxFileSizeBytes)
    }
}
