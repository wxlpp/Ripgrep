import XCTest
import ArgumentParser
@testable import OhMyGrepTool

final class ArgsParsingTests: XCTestCase {
    func testRequiredPattern() {
        XCTAssertThrowsError(try OhMyGrepArgs.parse([]))
    }
    func testPatternOnly() throws {
        let a = try OhMyGrepArgs.parse(["TODO"])
        XCTAssertEqual(a.pattern, "TODO")
        XCTAssertTrue(a.paths.isEmpty)
    }
    func testIgnoreCase() throws {
        let a = try OhMyGrepArgs.parse(["-i", "x"])
        XCTAssertTrue(a.ignoreCase)
    }
    func testSmartCaseDefaultOff() throws {
        let a = try OhMyGrepArgs.parse(["x"])
        XCTAssertFalse(a.smartCase)
    }
    func testGlobRepeats() throws {
        let a = try OhMyGrepArgs.parse(["x", "-g", "*.swift", "-g", "!*.test.swift"])
        XCTAssertEqual(a.glob, ["*.swift", "!*.test.swift"])
    }
    func testContextSetsBoth() throws {
        let a = try OhMyGrepArgs.parse(["x", "-C", "3"])
        XCTAssertEqual(a.context, 3)
    }
    func testJSONFlag() throws {
        let a = try OhMyGrepArgs.parse(["--json", "x"])
        XCTAssertTrue(a.json)
    }
}
