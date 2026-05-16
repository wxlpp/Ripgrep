import XCTest
@testable import RipgrepKitCore

final class SearchTests: XCTestCase {
    private func fixturePath() -> String {
        let url = Bundle.module.url(forResource: "mini", withExtension: nil, subdirectory: "Fixtures")!
        return url.path
    }

    func testFindsMatches() async throws {
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()])
        XCTAssertGreaterThan(r.matches.count, 0)
        XCTAssertTrue(r.matches.contains { $0.line.contains("TODO") })
    }

    func testRespectsGitignoreByDefault() async throws {
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()])
        XCTAssertFalse(r.matches.contains { $0.path.contains("ignored.txt") })
        XCTAssertFalse(r.matches.contains { $0.path.contains("/target/") })
    }

    func testHiddenFilesSkippedByDefault() async throws {
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()])
        XCTAssertFalse(r.matches.contains { $0.path.contains(".hidden") })
    }

    func testIncludeGlobFiltersToSwift() async throws {
        var opts = Ripgrep.Options(); opts.include = ["*.swift"]
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()], options: opts)
        XCTAssertTrue(r.matches.allSatisfy { $0.path.hasSuffix(".swift") })
    }

    func testInvalidPatternThrows() async throws {
        do {
            _ = try await Ripgrep.search(pattern: "[", in: [fixturePath()])
            XCTFail("expected throw")
        } catch let e as Ripgrep.Error {
            if case .invalidPattern = e { } else { XCTFail("wrong error: \(e)") }
        }
    }

    func testNegativeOptionsThrowsInvalidArguments() async throws {
        // A public Codable Options with a negative field must surface as a
        // recoverable Ripgrep.Error, NOT trap the host process (toFFI throws
        // synchronously before any FFI/walk, so no fixture is needed).
        var opts = Ripgrep.Options()
        opts.beforeContext = -1
        do {
            _ = try await Ripgrep.search(pattern: "x", in: ["."], options: opts)
            XCTFail("expected throw for negative beforeContext")
        } catch let e as Ripgrep.Error {
            guard case .invalidArguments = e else {
                return XCTFail("wrong error: \(e)")
            }
        }
    }

    func testOverUInt32MaxOptionsThrowsInvalidArguments() async throws {
        // Options is public Codable; a hand-constructed or JSON-decoded value
        // with a field exceeding UInt32.max must throw Ripgrep.Error.invalidArguments
        // rather than trap the host process via UInt32.init(_:) on a 64-bit Int.
        // Without the (0...Int(UInt32.max)) range guard, UInt32(5_000_000_000)
        // traps unconditionally at runtime — traps cannot be caught, so this test
        // would crash the test runner. The guard is what prevents the crash.
        let overMax: Int = 5_000_000_000  // > UInt32.max (4_294_967_295)

        for makeOpts: () -> Ripgrep.Options in [
            { var o = Ripgrep.Options(); o.maxMatches = overMax; return o },
            { var o = Ripgrep.Options(); o.maxFiles = overMax; return o },
            { var o = Ripgrep.Options(); o.beforeContext = overMax; return o },
            { var o = Ripgrep.Options(); o.afterContext = overMax; return o },
        ] {
            let opts = makeOpts()
            do {
                _ = try await Ripgrep.search(pattern: "x", in: ["."], options: opts)
                XCTFail("expected throw for over-UInt32.max field")
            } catch let e as Ripgrep.Error {
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
        var opts = Ripgrep.Options()
        opts.timeout = .seconds(-1)
        do {
            _ = try await Ripgrep.search(pattern: "x", in: ["."], options: opts)
            XCTFail("expected throw for negative timeout")
        } catch let e as Ripgrep.Error {
            guard case .invalidArguments = e else {
                return XCTFail("wrong error type: \(e)")
            }
        }

        // Non-regression: a small positive timeout must not throw invalidArguments.
        // (The search may complete normally or cancel — neither is an error here.)
        var validOpts = Ripgrep.Options()
        validOpts.timeout = .milliseconds(50)
        do {
            _ = try await Ripgrep.search(pattern: "x", in: ["."], options: validOpts)
        } catch let e as Ripgrep.Error {
            if case .invalidArguments = e {
                XCTFail("valid positive timeout must not throw invalidArguments")
            }
            // Other Ripgrep.Error variants (e.g. pathNotFound) are acceptable.
        }
    }
}
