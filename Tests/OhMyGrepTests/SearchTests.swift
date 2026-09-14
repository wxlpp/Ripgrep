import XCTest
@testable import OhMyGrep

final class SearchTests: XCTestCase {
    private func fixturePath() -> String {
        let url = Bundle.module.url(forResource: "mini", withExtension: nil, subdirectory: "Fixtures")!
        return url.path
    }

    func testFindsMatches() async throws {
        let r = try await OhMyGrep.search(pattern: "TODO", in: [fixturePath()])
        XCTAssertGreaterThan(r.matches.count, 0)
        XCTAssertTrue(r.matches.contains { $0.line.contains("TODO") })
    }

    /// A fresh temp directory, never inside a git checkout (unlike the bundled fixtures on macOS).
    private func makeTree(_ files: [String: String]) throws -> URL {
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("ohmygrep-search-\(UUID().uuidString)")
        for (name, content) in files {
            let url = dir.appendingPathComponent(name)
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try Data(content.utf8).write(to: url)
        }
        addTeardownBlock { try? FileManager.default.removeItem(at: dir) }
        return dir
    }

    private func names(_ r: OhMyGrep.SearchResult) -> [String] {
        Set(r.matches.map { URL(fileURLWithPath: $0.path).lastPathComponent }).sorted()
    }

    func testGitignoreOutsideRepoRequiresRequireGitFalse() async throws {
        let dir = try makeTree([
            ".gitignore": "git_ignored.txt\n", ".ignore": "dot_ignored.txt\n",
            "kept.txt": "TODO\n", "git_ignored.txt": "TODO\n", "dot_ignored.txt": "TODO\n",
        ])
        let rgDefault = try await OhMyGrep.search(pattern: "TODO", in: [dir.path])
        XCTAssertEqual(names(rgDefault), ["git_ignored.txt", "kept.txt"])

        var sandbox = OhMyGrep.Options(); sandbox.requireGit = false
        let noGit = try await OhMyGrep.search(pattern: "TODO", in: [dir.path], options: sandbox)
        XCTAssertEqual(names(noGit), ["kept.txt"])
    }

    func testEmptyPathsThrow() async throws {
        do {
            _ = try await OhMyGrep.search(pattern: "x", in: [])
            XCTFail("expected throw")
        } catch let e as OhMyGrep.Error {
            guard case .invalidArguments = e else { return XCTFail("wrong error: \(e)") }
        }
    }

    func testNamedBinaryFileSurfacesWarning() async throws {
        let dir = try makeTree(["blob.dat": "x\u{0}y TODO\n"])
        let file = dir.appendingPathComponent("blob.dat").path
        let r = try await OhMyGrep.search(pattern: "TODO", in: [file])
        XCTAssertTrue(r.matches.isEmpty)
        XCTAssertEqual(r.warnings.count, 1)
        XCTAssertEqual(r.warnings.first?.path, file)
        XCTAssertTrue(r.warnings.first?.message.hasPrefix("binary file matches") == true)
    }

    func testHiddenFilesSkippedByDefault() async throws {
        let r = try await OhMyGrep.search(pattern: "TODO", in: [fixturePath()])
        XCTAssertFalse(r.matches.contains { $0.path.contains(".hidden") })
    }

    func testIncludeGlobFiltersToSwift() async throws {
        var opts = OhMyGrep.Options(); opts.include = ["*.swift"]
        let r = try await OhMyGrep.search(pattern: "TODO", in: [fixturePath()], options: opts)
        XCTAssertTrue(r.matches.allSatisfy { $0.path.hasSuffix(".swift") })
    }

    func testInvalidPatternThrows() async throws {
        do {
            _ = try await OhMyGrep.search(pattern: "[", in: [fixturePath()])
            XCTFail("expected throw")
        } catch let e as OhMyGrep.Error {
            if case .invalidPattern = e { } else { XCTFail("wrong error: \(e)") }
        }
    }

    func testNegativeOptionsThrowsInvalidArguments() async throws {
        // A public Codable Options with a negative field must surface as a
        // recoverable OhMyGrep.Error, NOT trap the host process (toFFI throws
        // synchronously before any FFI/walk, so no fixture is needed).
        var opts = OhMyGrep.Options()
        opts.beforeContext = -1
        do {
            _ = try await OhMyGrep.search(pattern: "x", in: ["."], options: opts)
            XCTFail("expected throw for negative beforeContext")
        } catch let e as OhMyGrep.Error {
            guard case .invalidArguments = e else {
                return XCTFail("wrong error: \(e)")
            }
        }
    }

    func testOverUInt32MaxOptionsThrowsInvalidArguments() async throws {
        // Options is public Codable; a hand-constructed or JSON-decoded value
        // with a field exceeding UInt32.max must throw OhMyGrep.Error.invalidArguments
        // rather than trap the host process via UInt32.init(_:) on a 64-bit Int.
        // Without the (0...Int(UInt32.max)) range guard, UInt32(5_000_000_000)
        // traps unconditionally at runtime — traps cannot be caught, so this test
        // would crash the test runner. The guard is what prevents the crash.
        let overMax: Int = 5_000_000_000  // > UInt32.max (4_294_967_295)

        for makeOpts: () -> OhMyGrep.Options in [
            { var o = OhMyGrep.Options(); o.maxMatches = overMax; return o },
            { var o = OhMyGrep.Options(); o.maxFiles = overMax; return o },
            { var o = OhMyGrep.Options(); o.beforeContext = overMax; return o },
            { var o = OhMyGrep.Options(); o.afterContext = overMax; return o },
        ] {
            let opts = makeOpts()
            do {
                _ = try await OhMyGrep.search(pattern: "x", in: ["."], options: opts)
                XCTFail("expected throw for over-UInt32.max field")
            } catch let e as OhMyGrep.Error {
                guard case .invalidArguments = e else {
                    return XCTFail("wrong error type: \(e)")
                }
            }
        }
    }

    func testNegativeTimeoutThrowsInvalidArguments() async throws {
        // Options.timeout is public Codable; a negative Duration such as
        // .seconds(-1) previously produced UInt64(negative) = uncatchable host
        // trap. ffiMilliseconds() now guards with >= .zero and throws instead.
        // This test verifies the guard is reached: since a trap cannot be caught,
        // reaching the catch branch proves the guard (not a crash) handled it.
        var opts = OhMyGrep.Options()
        opts.timeout = .seconds(-1)
        do {
            _ = try await OhMyGrep.search(pattern: "x", in: ["."], options: opts)
            XCTFail("expected throw for negative timeout")
        } catch let e as OhMyGrep.Error {
            guard case .invalidArguments = e else {
                return XCTFail("wrong error type: \(e)")
            }
        }

        // Non-regression: a small positive timeout must not throw invalidArguments.
        // (The search may complete normally or cancel — neither is an error here.)
        var validOpts = OhMyGrep.Options()
        validOpts.timeout = .milliseconds(50)
        do {
            _ = try await OhMyGrep.search(pattern: "x", in: ["."], options: validOpts)
        } catch let e as OhMyGrep.Error {
            if case .invalidArguments = e {
                XCTFail("valid positive timeout must not throw invalidArguments")
            }
            // Other OhMyGrep.Error variants (e.g. pathNotFound) are acceptable.
        }
    }
}
