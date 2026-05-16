import XCTest
import ArgumentParser
@testable import RipgrepKitTool

final class ArgsParsingTests: XCTestCase {
    func testRequiredPattern() {
        XCTAssertThrowsError(try RipgrepArgs.parse([]))
    }
    func testPatternOnly() throws {
        let a = try RipgrepArgs.parse(["TODO"])
        XCTAssertEqual(a.pattern, "TODO")
        XCTAssertTrue(a.paths.isEmpty)
    }
    func testIgnoreCase() throws {
        let a = try RipgrepArgs.parse(["-i", "x"])
        XCTAssertTrue(a.ignoreCase)
    }
    func testSmartCaseDefaultOff() throws {
        let a = try RipgrepArgs.parse(["x"])
        XCTAssertFalse(a.smartCase)
    }
    func testGlobRepeats() throws {
        let a = try RipgrepArgs.parse(["x", "-g", "*.swift", "-g", "!*.test.swift"])
        XCTAssertEqual(a.glob, ["*.swift", "!*.test.swift"])
    }
    func testContextSetsBoth() throws {
        let a = try RipgrepArgs.parse(["x", "-C", "3"])
        XCTAssertEqual(a.context, 3)
    }
    func testJSONFlag() throws {
        let a = try RipgrepArgs.parse(["--json", "x"])
        XCTAssertTrue(a.json)
    }
}
