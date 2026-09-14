import XCTest
@testable import OhMyGrep

final class OptionsTests: XCTestCase {
    func testDefaultsMatchRipgrepCLI() {
        let o = OhMyGrep.Options()
        XCTAssertFalse(o.caseInsensitive)
        XCTAssertFalse(o.smartCase)        // matches rg CLI default
        XCTAssertFalse(o.multiline)
        XCTAssertTrue(o.respectGitignore)
        XCTAssertTrue(o.requireGit)
        XCTAssertFalse(o.includeHidden)
        XCTAssertEqual(o.beforeContext, 0)
        XCTAssertEqual(o.afterContext, 0)
        XCTAssertEqual(o.maxMatches, 10_000)
        XCTAssertEqual(o.maxColumns, 4096)
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
            respectGitignore: false, requireGit: false, includeHidden: true,
            beforeContext: 1, afterContext: 2,
            maxMatches: 3, maxFiles: 4, maxFileSizeBytes: 5,
            timeout: .milliseconds(6), searchBinary: true)
        let d = try JSONDecoder().decode(OhMyGrep.Options.self, from: JSONEncoder().encode(o))
        XCTAssertEqual(d, o)
        XCTAssertNotEqual(o, OhMyGrep.Options())
    }

    func testExplicitNullLimitMeansUnlimited() throws {
        let json = Data(#"{"maxMatches": null, "maxColumns": null}"#.utf8)
        let o = try JSONDecoder().decode(OhMyGrep.Options.self, from: json)
        XCTAssertNil(o.maxMatches)
        XCTAssertNil(o.maxColumns)
    }

    func testMaxColumnsMustBePositive() async throws {
        var o = OhMyGrep.Options(); o.maxColumns = 0
        do {
            _ = try await OhMyGrep.search(pattern: "x", in: ["/"], options: o)
            XCTFail("expected throw")
        } catch let e as OhMyGrep.Error {
            guard case .invalidArguments = e else { return XCTFail("wrong error: \(e)") }
        }
    }

    func testDecodingMissingKeysUsesDefaults() throws {
        let o = try JSONDecoder().decode(OhMyGrep.Options.self, from: Data(#"{"beforeContext": 1}"#.utf8))
        XCTAssertEqual(o.beforeContext, 1)
        XCTAssertTrue(o.respectGitignore)
        XCTAssertFalse(o.searchBinary)
        XCTAssertEqual(o.maxMatches, 10_000)
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
        XCTAssertEqual(decoded.maxMatches, 10_000)
        XCTAssertNil(decoded.maxFiles)
        XCTAssertNil(decoded.maxFileSizeBytes)
    }
}
