import RipgrepKitFFI

extension Ripgrep {
    public enum Error: Swift.Error, Sendable {
        case invalidArguments(message: String)  // labeled: parser-generated, not FFI-bridged
        case invalidPattern(String)
        case pathNotFound(String)
        case io(String)
        case internalPanic(String)

        public var message: String {
            switch self {
            case .invalidArguments(let m): return m
            case .invalidPattern(let p):   return "invalid regex: \(p)"
            case .pathNotFound(let p):     return "path not found: \(p)"
            case .io(let m):               return "io error: \(m)"
            case .internalPanic(let m):    return "internal panic: \(m)"
            }
        }

        /// Maps a UniFFI-generated RipgrepError to our public Error.
        /// Case names (.InvalidPattern etc.) are PascalCase as emitted by UniFFI
        /// for ripgrep_core 0.1.0. If UniFFI regenerates with different casing,
        /// update this switch to match the enum in Sources/RipgrepKitFFI/RipgrepCore.swift.
        static func from(_ ffi: RipgrepError) -> Ripgrep.Error {
            switch ffi {
            case .InvalidPattern(let s): return .invalidPattern(s)
            case .PathNotFound(let s):   return .pathNotFound(s)
            case .Io(let s):             return .io(s)
            case .InternalPanic(let s):  return .internalPanic(s)
            }
        }
    }
}
