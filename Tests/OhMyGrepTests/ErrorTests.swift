import XCTest
@testable import OhMyGrep

final class ErrorTests: XCTestCase {
    func testErrorMessageRendering() {
        let e = OhMyGrep.Error.invalidPattern("[")
        XCTAssertTrue(e.message.contains("invalid"))
        XCTAssertTrue(e.message.contains("["))
    }

    func testInvalidArgumentsCarriesMessage() {
        let e = OhMyGrep.Error.invalidArguments(message: "missing pattern")
        XCTAssertEqual(e.message, "missing pattern")
    }
}
