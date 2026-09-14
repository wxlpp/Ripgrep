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
        XCTAssertFalse(o.searchBinary)
    }

    func testCodableRoundtripAllFieldsNonDefault() throws {
        let o = OhMyGrep.Options(
            caseInsensitive: true, smartCase: true, multiline: true,
            include: ["*.swift"], exclude: ["*.md"], fileTypes: ["rust"],
            respectGitignore: false, includeHidden: true,
            beforeContext: 1, afterContext: 2,
            maxMatches: 3, maxFiles: 4, maxFileSizeBytes: 5,
            timeout: .milliseconds(6), searchBinary: true)
        let d = try JSONDecoder().decode(OhMyGrep.Options.self, from: JSONEncoder().encode(o))
        XCTAssertEqual(d, o)
        XCTAssertNotEqual(o, OhMyGrep.Options())
    }

    func testDecodingMissingKeysUsesDefaults() throws {
        let o = try JSONDecoder().decode(OhMyGrep.Options.self, from: Data(#"{"beforeContext": 1}"#.utf8))
        XCTAssertEqual(o.beforeContext, 1)
        XCTAssertTrue(o.respectGitignore)
        XCTAssertFalse(o.searchBinary)
        XCTAssertNil(o.maxMatches)
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
