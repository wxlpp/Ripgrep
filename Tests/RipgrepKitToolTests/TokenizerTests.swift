import XCTest
@testable import RipgrepKitTool
@testable import RipgrepKitCore

final class TokenizerTests: XCTestCase {
    func testSimpleSplit() throws {
        XCTAssertEqual(try Tokenizer.tokenize("foo bar baz"), ["foo", "bar", "baz"])
    }
    func testDoubleQuotes() throws {
        XCTAssertEqual(try Tokenizer.tokenize("\"hello world\" foo"), ["hello world", "foo"])
    }
    func testSingleQuotes() throws {
        XCTAssertEqual(try Tokenizer.tokenize(#"'func\s+\w+' x"#), ["func\\s+\\w+", "x"])
    }
    func testBackslashEscape() throws {
        XCTAssertEqual(try Tokenizer.tokenize(#"a\ b c"#), ["a b", "c"])
    }
    func testFlagEqualsValueSingleToken() throws {
        XCTAssertEqual(try Tokenizer.tokenize(#"--glob='*.swift' src"#),
                       ["--glob=*.swift", "src"])
    }
    func testFlagEqualsValueDoubleQuoted() throws {
        XCTAssertEqual(try Tokenizer.tokenize(#"--glob="hello world""#),
                       ["--glob=hello world"])
    }
    func testUnbalancedQuoteThrows() {
        XCTAssertThrowsError(try Tokenizer.tokenize(#""unclosed"#)) { e in
            guard let e = e as? Ripgrep.Error else { return XCTFail() }
            if case .invalidArguments = e {} else { XCTFail() }
        }
    }
    func testDanglingBackslashThrows() {
        XCTAssertThrowsError(try Tokenizer.tokenize(#"foo\"#)) { e in
            guard let e = e as? Ripgrep.Error else { return XCTFail() }
            if case .invalidArguments = e {} else { XCTFail() }
        }
    }
    func testEmptyInputReturnsEmpty() throws {
        XCTAssertEqual(try Tokenizer.tokenize(""), [])
        XCTAssertEqual(try Tokenizer.tokenize("   "), [])
    }

    func testEmptyQuotedProducesEmptyToken() throws {
        // Intentional, shell-faithful: an explicit empty quoted region IS a real
        // empty-string token (so e.g. `rg -g ''` reaches the parser as an empty
        // glob to reject, rather than being silently dropped). Pinned here.
        XCTAssertEqual(try Tokenizer.tokenize("\"\" foo"), ["", "foo"])
        XCTAssertEqual(try Tokenizer.tokenize("''"), [""])
    }
}
