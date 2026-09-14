import Foundation
import XCTest
@testable import OhMyGrep

final class StreamTests: XCTestCase {
    private var dir: URL!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("ohmygrep-stream-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let body = (0..<50).map { "HIT \($0)\nfiller\n" }.joined()
        for i in 0..<40 {
            try Data(body.utf8).write(to: dir.appendingPathComponent(String(format: "f%02d.txt", i)))
        }
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: dir)
    }

    private func unlimited() -> OhMyGrep.Options {
        var o = OhMyGrep.Options(); o.maxMatches = nil; return o
    }

    private actor Collector {
        var matches: [OhMyGrep.Match] = []
        func add(_ m: OhMyGrep.Match) { matches.append(m) }
        var count: Int { matches.count }
    }

    private func key(_ m: OhMyGrep.Match) -> String { "\(m.path):\(m.lineNumber)" }

    func testStreamYieldsSameMatchesAsSearch() async throws {
        let collector = Collector()
        let summary = try await OhMyGrep.stream(pattern: "HIT", in: [dir.path], options: unlimited()) {
            await collector.add($0)
        }
        let streamed = await collector.matches.map(key).sorted()
        let searched = try await OhMyGrep.search(pattern: "HIT", in: [dir.path], options: unlimited())
        XCTAssertEqual(streamed, searched.matches.map(key).sorted())
        XCTAssertEqual(streamed.count, 2000)
        XCTAssertEqual(summary.filesSearched, 40)
        XCTAssertFalse(summary.truncated)
        XCTAssertFalse(summary.cancelled)
    }

    func testStreamHonorsMatchLimit() async throws {
        var options = OhMyGrep.Options(); options.maxMatches = 25
        let collector = Collector()
        let summary = try await OhMyGrep.stream(pattern: "HIT", in: [dir.path], options: options) {
            await collector.add($0)
        }
        let count = await collector.count
        XCTAssertEqual(count, 25)
        XCTAssertTrue(summary.truncated)
    }

    func testCancellingTheTaskStopsTheStream() async throws {
        let collector = Collector()
        let options = unlimited()
        let path = dir.path
        let task = Task {
            try await OhMyGrep.stream(pattern: "HIT", in: [path], options: options) { match in
                await collector.add(match)
                try? await Task.sleep(for: .milliseconds(2))
            }
        }
        while await collector.count < 10 { try await Task.sleep(for: .milliseconds(5)) }
        task.cancel()
        let summary = try await task.value
        XCTAssertTrue(summary.cancelled)
        let count = await collector.count
        XCTAssertLessThan(count, 2000)
    }

    private struct Stop: Swift.Error {}

    func testErrorFromOnMatchIsRethrownAndSearchingStillWorks() async throws {
        do {
            try await OhMyGrep.stream(pattern: "HIT", in: [dir.path], options: unlimited()) { _ in throw Stop() }
            XCTFail("expected throw")
        } catch is Stop {}
        let again = try await OhMyGrep.search(pattern: "HIT 7$", in: [dir.path])
        XCTAssertEqual(again.matches.count, 40)
    }

    func testRequestErrorsAreThrown() async throws {
        do {
            try await OhMyGrep.stream(pattern: "[", in: [dir.path]) { _ in }
            XCTFail("expected throw")
        } catch let e as OhMyGrep.Error {
            guard case .invalidPattern = e else { return XCTFail("wrong error: \(e)") }
        }
    }

    func testSummaryCarriesWarnings() async throws {
        let blob = dir.appendingPathComponent("blob.dat")
        try Data("x\u{0}y HIT\n".utf8).write(to: blob)
        let summary = try await OhMyGrep.stream(pattern: "HIT", in: [blob.path]) { _ in }
        XCTAssertEqual(summary.warnings.map(\.path), [blob.path])
    }

    func testLongLinesAreWindowedAroundTheMatch() async throws {
        let long = String(repeating: "a", count: 10_000) + "NEEDLE" + String(repeating: "b", count: 10_000)
        let file = dir.appendingPathComponent("long.txt")
        try Data((long + "\n").utf8).write(to: file)
        var options = OhMyGrep.Options(); options.maxColumns = 100
        let r = try await OhMyGrep.search(pattern: "NEEDLE", in: [file.path], options: options)
        let m = try XCTUnwrap(r.matches.first)
        XCTAssertTrue(m.lineTruncated)
        XCTAssertEqual(m.lineOffset, 10_000 - 25)
        let s = try XCTUnwrap(m.submatches.first)
        let utf8 = Array(m.line.utf8)
        XCTAssertEqual(String(decoding: utf8[s.start..<s.end], as: UTF8.self), "NEEDLE")
    }
}
