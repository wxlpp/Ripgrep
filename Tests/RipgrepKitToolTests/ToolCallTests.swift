import XCTest
@testable import RipgrepKitTool
@testable import RipgrepKitCore

final class ToolCallTests: XCTestCase {
    private func fixturePath() -> String {
        Bundle.module.url(forResource: "mini", withExtension: nil, subdirectory: "Fixtures")!.path
    }

    func testToolSchemaIsValidJSONWithRequiredKeys() throws {
        let data = Ripgrep.toolSchema.data(using: .utf8)!
        let obj = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(obj?["name"] as? String, "ripgrep")
        XCTAssertNotNil(obj?["description"])
        XCTAssertNotNil(obj?["input_schema"])
    }

    func testToolInputCodableRoundtrip() throws {
        let i = Ripgrep.ToolInput(args: "TODO src/")
        let data = try JSONEncoder().encode(i)
        let decoded = try JSONDecoder().decode(Ripgrep.ToolInput.self, from: data)
        XCTAssertEqual(decoded.args, "TODO src/")
    }

    func testHandleToolCallSucceeds() async throws {
        let input = Ripgrep.ToolInput(args: "TODO \(fixturePath())")
        let out = try await Ripgrep.handleToolCall(input)
        XCTAssertTrue(out.contains("TODO"))
    }

    func testHandleToolCallReturnsErrorString() async throws {
        let input = Ripgrep.ToolInput(args: "[ \(fixturePath())")  // bad regex
        let out = try await Ripgrep.handleToolCall(input)
        XCTAssertTrue(out.hasPrefix("ERROR:"))
    }
}
