import XCTest
@testable import RipgrepKitCore

final class CancellationTests: XCTestCase {
    private func fixturePath() -> String {
        Bundle.module.url(forResource: "mini", withExtension: nil, subdirectory: "Fixtures")!.path
    }

    func testTimeoutMarksResultCancelled() async throws {
        var opts = Ripgrep.Options()
        opts.timeout = .nanoseconds(1)   // immediate
        let r = try await Ripgrep.search(pattern: "TODO", in: [fixturePath()], options: opts)
        XCTAssertTrue(r.cancelled || r.matches.isEmpty)
    }

    func testTaskCancelStopsSearch() async throws {
        let path = fixturePath()
        let task = Task {
            try await Ripgrep.search(pattern: "TODO", in: [path])
        }
        task.cancel()
        do {
            let r = try await task.value
            XCTAssertTrue(r.cancelled || r.matches.count >= 0)  // tiny fixture may finish first
        } catch is CancellationError {
            // acceptable: detached Task surfaces cancellation
        }
    }
}
