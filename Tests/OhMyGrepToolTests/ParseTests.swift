import XCTest
@testable import OhMyGrepTool
@testable import OhMyGrep

final class ParseTests: XCTestCase {
    func testBasicParse() throws {
        let p = try OhMyGrep.parse("TODO src/")
        XCTAssertEqual(p.pattern, "TODO")
        XCTAssertEqual(p.paths, ["src/"])
        XCTAssertEqual(p.outputFormat, .text)
    }

    func testGlobSplitByPrefix() throws {
        let p = try OhMyGrep.parse("x -g '*.swift' -g '!*.test.swift'")
        XCTAssertEqual(p.options.include, ["*.swift"])
        XCTAssertEqual(p.options.exclude, ["*.test.swift"])
    }

    func testNoRequireGitFlag() throws {
        XCTAssertTrue(try OhMyGrep.parse("x").options.requireGit)
        XCTAssertFalse(try OhMyGrep.parse("x --no-require-git").options.requireGit)
    }

    func testNoIgnoreInvertsGitignore() throws {
        let p = try OhMyGrep.parse("x --no-ignore")
        XCTAssertFalse(p.options.respectGitignore)
    }

    func testContextAppliesToBothWhenUnset() throws {
        let p = try OhMyGrep.parse("x -C 2")
        XCTAssertEqual(p.options.beforeContext, 2)
        XCTAssertEqual(p.options.afterContext, 2)
    }

    func testExplicitAfterOverridesContext() throws {
        let p = try OhMyGrep.parse("x -C 2 -A 5")
        XCTAssertEqual(p.options.afterContext, 5)
        XCTAssertEqual(p.options.beforeContext, 2)
    }

    func testExplicitZeroOverridesContext() throws {
        let p = try OhMyGrep.parse("x -C 3 -A 0")
        XCTAssertEqual(p.options.afterContext, 0)
        XCTAssertEqual(p.options.beforeContext, 3)
    }

    func testTextFlagSearchesBinary() throws {
        XCTAssertFalse(try OhMyGrep.parse("x").options.searchBinary)
        XCTAssertTrue(try OhMyGrep.parse("-a x").options.searchBinary)
        XCTAssertTrue(try OhMyGrep.parse("--text x").options.searchBinary)
    }

    func testEmptyPathsDefaultsToCwd() throws {
        let p = try OhMyGrep.parse("x")
        XCTAssertEqual(p.paths, ["."])
    }

    func testJsonFlagSetsOutputFormat() throws {
        let p = try OhMyGrep.parse("--json x")
        XCTAssertEqual(p.outputFormat, .jsonLines)
    }

    func testInvalidArgsThrowWithUsage() {
        XCTAssertThrowsError(try OhMyGrep.parse("")) { e in
            guard let e = e as? OhMyGrep.Error else { return XCTFail() }
            if case .invalidArguments(let m) = e {
                XCTAssertTrue(m.lowercased().contains("usage") || m.lowercased().contains("missing"))
            } else { XCTFail() }
        }
    }

    func testMaxFilesizeParsing() throws {
        XCTAssertEqual(try OhMyGrep.parse("x --max-filesize 5M").options.maxFileSizeBytes, 5 * 1024 * 1024)
        XCTAssertEqual(try OhMyGrep.parse("x --max-filesize 1K").options.maxFileSizeBytes, 1024)
        XCTAssertEqual(try OhMyGrep.parse("x --max-filesize 100").options.maxFileSizeBytes, 100)
        XCTAssertNil(try OhMyGrep.parse("x").options.maxFileSizeBytes)  // absent → no limit
        // present but unparseable → error (not silently "no limit")
        for bad in ["x --max-filesize bad", "x --max-filesize=-5M", "x --max-filesize 5X"] {
            XCTAssertThrowsError(try OhMyGrep.parse(bad)) { e in
                guard let e = e as? OhMyGrep.Error, case .invalidArguments = e else {
                    return XCTFail("expected .invalidArguments for: \(bad)")
                }
            }
        }
    }

    func testCaseFlagsMapIntoOptions() throws {
        let p = try OhMyGrep.parse("-i -S -U pat")
        XCTAssertTrue(p.options.caseInsensitive)
        XCTAssertTrue(p.options.smartCase)
        XCTAssertTrue(p.options.multiline)
    }
}

final class RunTests: XCTestCase {
    private func fixturePath() -> String {
        Bundle.module.url(forResource: "mini", withExtension: nil, subdirectory: "Fixtures")!.path
    }

    func testRunStringEndToEnd() async throws {
        let p = fixturePath()
        let out = try await OhMyGrep.run("TODO \(p)")
        XCTAssertTrue(out.contains("TODO"))
    }

    func testRunJSONOutput() async throws {
        let p = fixturePath()
        let out = try await OhMyGrep.run("--json TODO \(p)")
        let firstLine = out.split(separator: "\n").first.map(String.init) ?? ""
        XCTAssertTrue(firstLine.hasPrefix("{"))
    }
}
