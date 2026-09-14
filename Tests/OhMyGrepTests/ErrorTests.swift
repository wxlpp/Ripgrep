import XCTest
@testable import OhMyGrep

final class ErrorTests: XCTestCase {
    func testErrorMessageRendering() {
        let e = OhMyGrep.Error.invalidPattern("[")
        XCTAssertTrue(e.message.contains("invalid"))
        XCTAssertTrue(e.message.contains("["))
    }

    func testFFIErrorPrefixIsNotDoubled() async throws {
        do {
            _ = try await OhMyGrep.search(pattern: "x", in: ["/nonexistent-ohmygrep-path"])
            XCTFail("expected throw")
        } catch let e as OhMyGrep.Error {
            XCTAssertEqual(e.message, "path not found: /nonexistent-ohmygrep-path")
        }
    }

    func testInvalidArgumentsCarriesMessage() {
        let e = OhMyGrep.Error.invalidArguments(message: "missing pattern")
        XCTAssertEqual(e.message, "missing pattern")
    }
}
