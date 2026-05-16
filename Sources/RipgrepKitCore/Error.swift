import RipgrepKitFFI

extension Ripgrep {
    public enum Error: Swift.Error, Sendable {
        case invalidArguments(message: String)
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

        /// Maps a UniFFI-generated error to our public Error.
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
