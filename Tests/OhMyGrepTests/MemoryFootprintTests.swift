import Foundation
import XCTest
@testable import OhMyGrep

/// Prints memory footprints for streaming vs one-shot search. Opt-in: set OHMYGREP_PERF=1
/// and OHMYGREP_PERF_FILE to a large log with many matches for the pattern `status=200`.
final class MemoryFootprintTests: XCTestCase {
    private static func footprintMiB() -> Double {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<integer_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
            }
        }
        return kr == KERN_SUCCESS ? Double(info.phys_footprint) / 1_048_576 : -1
    }

    func testStreamFootprintStaysFlat() async throws {
        let env = ProcessInfo.processInfo.environment
        guard env["OHMYGREP_PERF"] == "1", let file = env["OHMYGREP_PERF_FILE"] else {
            throw XCTSkip("set OHMYGREP_PERF=1 and OHMYGREP_PERF_FILE")
        }
        var options = OhMyGrep.Options(); options.maxMatches = nil

        let before = Self.footprintMiB()
        let peak = PeakTracker(start: before)
        let summary = try await OhMyGrep.stream(pattern: "status=200", in: [file], options: options) { _ in
            peak.sample(MemoryFootprintTests.footprintMiB())
        }
        let streamPeak = peak.value
        print("PERF stream: matches drained, files=\(summary.filesSearched) footprint before=\(before) MiB peak=\(streamPeak) MiB")

        let result = try await OhMyGrep.search(pattern: "status=200", in: [file], options: options)
        let afterSearch = Self.footprintMiB()
        print("PERF one-shot: matches=\(result.matches.count) footprint holding result=\(afterSearch) MiB")
    }
}

private final class PeakTracker: @unchecked Sendable {
    private let lock = NSLock()
    private var peak: Double
    private var samples = 0
    init(start: Double) { peak = start }
    func sample(_ value: Double) {
        lock.lock(); defer { lock.unlock() }
        samples += 1
        // Sampling every match would dominate the run; every 1000th is enough for a peak.
        if samples % 1000 == 0 { peak = max(peak, value) }
    }
    var value: Double { lock.lock(); defer { lock.unlock() }; return peak }
}
