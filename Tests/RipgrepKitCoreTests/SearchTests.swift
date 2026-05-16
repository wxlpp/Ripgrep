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
}
