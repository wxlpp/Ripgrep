import Foundation

extension OhMyGrep.Match {
    /// `submatches` as ranges of `line`, for highlighting or slicing.
    ///
    /// Submatch offsets count bytes of the file's raw line. They line up with `line.utf8`
    /// up to the first U+FFFD, which marks invalid UTF-8 replaced during decoding; a
    /// submatch ending after that point, or not on a Unicode scalar boundary, is omitted.
    public var submatchRanges: [Range<String.Index>] {
        let utf8 = line.utf8
        let validBytes = line.unicodeScalars.firstIndex(of: "\u{FFFD}")
            .map { utf8.distance(from: utf8.startIndex, to: $0) } ?? utf8.count
        return submatches.compactMap { submatch in
            guard 0 <= submatch.start, submatch.start <= submatch.end, submatch.end <= validBytes else {
                return nil
            }
            let lower = utf8.index(utf8.startIndex, offsetBy: submatch.start)
            let upper = utf8.index(utf8.startIndex, offsetBy: submatch.end)
            guard lower.samePosition(in: line.unicodeScalars) != nil,
                  upper.samePosition(in: line.unicodeScalars) != nil else {
                return nil
            }
            return lower..<upper
        }
    }
}
