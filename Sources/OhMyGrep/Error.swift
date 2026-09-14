import OhMyGrepFFI

extension OhMyGrep {
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

        /// Maps the UniFFI error. `flat_error` delivers Rust's rendered Display
        /// ("path not found: /x"), so the Rust prefix is removed before re-wrapping.
        static func from(_ ffi: OhMyGrepError) -> OhMyGrep.Error {
            func body(_ s: String, _ prefix: String) -> String {
                s.hasPrefix(prefix) ? String(s.dropFirst(prefix.count)) : s
            }
            switch ffi {
            case .InvalidPattern(let s):   return .invalidPattern(body(s, "invalid regex: "))
            case .InvalidArguments(let s): return .invalidArguments(message: body(s, "invalid arguments: "))
            case .PathNotFound(let s):     return .pathNotFound(body(s, "path not found: "))
            case .Io(let s):               return .io(body(s, "io error: "))
            case .InternalPanic(let s):    return .internalPanic(body(s, "internal panic: "))
            }
        }
    }
}
