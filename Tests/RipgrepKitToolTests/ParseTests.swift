import XCTest
@testable import RipgrepKitTool
@testable import RipgrepKitCore

final class ParseTests: XCTestCase {
    func testBasicParse() throws {
        let p = try Ripgrep.parse("TODO src/")
        XCTAssertEqual(p.pattern, "TODO")
        XCTAssertEqual(p.paths, ["src/"])
        XCTAssertEqual(p.outputFormat, .text)
    }

    func testGlobSplitByPrefix() throws {
        let p = try Ripgrep.parse("x -g '*.swift' -g '!*.test.swift'")
        XCTAssertEqual(p.options.include, ["*.swift"])
        XCTAssertEqual(p.options.exclude, ["*.test.swift"])
    }

    func testNoIgnoreInvertsGitignore() throws {
        let p = try Ripgrep.parse("x --no-ignore")
        XCTAssertFalse(p.options.respectGitignore)
    }

    func testContextAppliesToBothWhenUnset() throws {
        let p = try Ripgrep.parse("x -C 2")
        XCTAssertEqual(p.options.beforeContext, 2)
        XCTAssertEqual(p.options.afterContext, 2)
    }

    func testExplicitAfterOverridesContext() throws {
        let p = try Ripgrep.parse("x -C 2 -A 5")
        XCTAssertEqual(p.options.afterContext, 5)
        XCTAssertEqual(p.options.beforeContext, 2)
    }

    func testEmptyPathsDefaultsToCwd() throws {
        let p = try Ripgrep.parse("x")
        XCTAssertEqual(p.paths, ["."])
    }

    func testJsonFlagSetsOutputFormat() throws {
        let p = try Ripgrep.parse("--json x")
        XCTAssertEqual(p.outputFormat, .jsonLines)
    }

    func testInvalidArgsThrowWithUsage() {
        XCTAssertThrowsError(try Ripgrep.parse("")) { e in
            guard let e = e as? Ripgrep.Error else { return XCTFail() }
            if case .invalidArguments(let m) = e {
                XCTAssertTrue(m.lowercased().contains("usage") || m.lowercased().contains("missing"))
            } else { XCTFail() }
        }
    }

    func testMaxFilesizeParsing() throws {
        XCTAssertEqual(try Ripgrep.parse("x --max-filesize 5M").options.maxFileSizeBytes, 5 * 1024 * 1024)
        XCTAssertEqual(try Ripgrep.parse("x --max-filesize 1K").options.maxFileSizeBytes, 1024)
        XCTAssertEqual(try Ripgrep.parse("x --max-filesize 100").options.maxFileSizeBytes, 100)
        XCTAssertNil(try Ripgrep.parse("x --max-filesize bad").options.maxFileSizeBytes)
        XCTAssertNil(try Ripgrep.parse("x --max-filesize=-5M").options.maxFileSizeBytes)
    }

    func testCaseFlagsMapIntoOptions() throws {
        let p = try Ripgrep.parse("-i -S -U pat")
        XCTAssertTrue(p.options.caseInsensitive)
        XCTAssertTrue(p.options.smartCase)
        XCTAssertTrue(p.options.multiline)
    }
}
