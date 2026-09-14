import XCTest
@testable import OhMyGrepTool
@testable import OhMyGrep

final class ToolCallTests: XCTestCase {
    private func fixturePath() -> String {
        Bundle.module.url(forResource: "mini", withExtension: nil, subdirectory: "Fixtures")!.path
    }

    func testToolSchemaIsValidJSONWithRequiredKeys() throws {
        let data = OhMyGrep.toolSchema.data(using: .utf8)!
        let obj = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(obj?["name"] as? String, "oh_my_grep")
        XCTAssertNotNil(obj?["description"])
        XCTAssertNotNil(obj?["input_schema"])
    }

    func testToolInputCodableRoundtrip() throws {
        let i = OhMyGrep.ToolInput(args: "TODO src/")
        let data = try JSONEncoder().encode(i)
        let decoded = try JSONDecoder().decode(OhMyGrep.ToolInput.self, from: data)
        XCTAssertEqual(decoded.args, "TODO src/")
    }

    func testHandleToolCallSucceeds() async throws {
        let input = OhMyGrep.ToolInput(args: "TODO \(fixturePath())")
        let out = try await OhMyGrep.handleToolCall(input)
        XCTAssertTrue(out.contains("TODO"))
    }

    func testHandleToolCallReturnsErrorString() async throws {
        let input = OhMyGrep.ToolInput(args: "[ \(fixturePath())")  // bad regex
        let out = try await OhMyGrep.handleToolCall(input)
        XCTAssertTrue(out.hasPrefix("ERROR:"))
    }
}
