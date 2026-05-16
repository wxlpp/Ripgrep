import XCTest
@testable import RipgrepKitCore

final class ErrorTests: XCTestCase {
    func testErrorMessageRendering() {
        let e = Ripgrep.Error.invalidPattern("[")
        XCTAssertTrue(e.message.contains("invalid"))
        XCTAssertTrue(e.message.contains("["))
    }

    func testInvalidArgumentsCarriesMessage() {
        let e = Ripgrep.Error.invalidArguments(message: "missing pattern")
        XCTAssertEqual(e.message, "missing pattern")
    }
}
