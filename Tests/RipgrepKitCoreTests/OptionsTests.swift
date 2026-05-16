import XCTest
@testable import RipgrepKitCore

final class OptionsTests: XCTestCase {
    func testDefaultsMatchRipgrepCLI() {
        let o = Ripgrep.Options()
        XCTAssertFalse(o.caseInsensitive)
        XCTAssertFalse(o.smartCase)        // matches rg CLI default
        XCTAssertFalse(o.multiline)
        XCTAssertTrue(o.respectGitignore)
        XCTAssertFalse(o.includeHidden)
        XCTAssertEqual(o.beforeContext, 0)
        XCTAssertEqual(o.afterContext, 0)
        XCTAssertNil(o.maxMatches)
        XCTAssertNil(o.timeout)
    }

    func testCodableRoundtrip() throws {
        var o = Ripgrep.Options()
        o.beforeContext = 2
        o.include = ["*.swift"]
        o.timeout = .milliseconds(500)
        let data = try JSONEncoder().encode(o)
        let decoded = try JSONDecoder().decode(Ripgrep.Options.self, from: data)
        XCTAssertEqual(decoded.beforeContext, 2)
        XCTAssertEqual(decoded.include, ["*.swift"])
    }
}
