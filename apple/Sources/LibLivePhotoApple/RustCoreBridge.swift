import Foundation
#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

public struct CoreOutputResource {
    public let name: String
    public let mime: String
    public let resourceID: Int64?
    public let bytes: Data
}
public struct CoreOperationResult {
    public let success: Bool
    /// Authoritative UTF-8 JSON exactly as emitted by the Rust core.
    public let headerJSON: Data
    public let files: [CoreOutputResource]
    public func inspectionManifest() throws -> InspectionManifest {
        struct Envelope: Decodable { let metadata: InspectionManifest }
        return try JSONDecoder().decode(Envelope.self, from: headerJSON).metadata
    }
    fileprivate init(envelope: Data) throws {
        struct Reader {
            let data: Data; var offset = 0
            mutating func integer(_ count: Int) throws -> UInt64 {
                guard count <= data.count - offset else { throw LivePhotoAdapterError.nativeFailure("truncated native integer") }
                var value: UInt64 = 0
                for byte in data[offset..<offset+count] { value = value << 8 | UInt64(byte) }
                offset += count; return value
            }
            mutating func field(max: Int) throws -> Data {
                let length = try integer(4)
                guard length <= UInt64(max), length <= UInt64(data.count - offset) else { throw LivePhotoAdapterError.inputLimit }
                let end = offset + Int(length); defer { offset = end }
                return data.subdata(in: offset..<end)
            }
        }
        var reader = Reader(data: envelope)
        guard try reader.integer(4) == 0x4c4c5031 else { throw LivePhotoAdapterError.nativeFailure("native LLP1 protocol mismatch") }
        let header = try reader.field(max: 64*1024*1024)
        struct Status: Decodable { let success: Bool }
        success = try JSONDecoder().decode(Status.self, from: header).success
        headerJSON = header
        let count = try reader.integer(4); guard count <= 4096 else { throw LivePhotoAdapterError.inputLimit }
        var files: [CoreOutputResource] = []
        for _ in 0..<count {
            guard let mime = String(data: try reader.field(max: 4096), encoding: .utf8), let name = String(data: try reader.field(max: 4096), encoding: .utf8) else { throw LivePhotoAdapterError.nativeFailure("invalid native output text") }
            let id = Int64(bitPattern: try reader.integer(8)); guard id >= -1 else { throw LivePhotoAdapterError.nativeFailure("invalid native resource ID") }
            files.append(CoreOutputResource(name: name, mime: mime, resourceID: id == -1 ? nil : id, bytes: try reader.field(max: 256*1024*1024)))
        }
        guard reader.offset == envelope.count else { throw LivePhotoAdapterError.nativeFailure("trailing native envelope data") }
        self.files = files
    }
}

#if canImport(Darwin) || canImport(Glibc)
/// Real C ABI client. The host supplies its locally built/signed native library;
/// this class never downloads code or loads a library from a remote URL.
public final class RustCoreBridge {
    private typealias Process = @convention(c) (UnsafePointer<UInt8>?, UInt, UnsafePointer<UInt8>?, UInt, UnsafePointer<UInt8>?, UInt) -> UnsafeMutableRawPointer?
    private typealias Bytes = @convention(c) (UnsafeRawPointer?) -> UnsafePointer<UInt8>?
    private typealias Length = @convention(c) (UnsafeRawPointer?) -> UInt
    private typealias Free = @convention(c) (UnsafeMutableRawPointer?) -> Void
    private typealias Version = @convention(c) () -> UInt32
    private let library: UnsafeMutableRawPointer
    private let process: Process
    private let responseBytes: Bytes
    private let responseLength: Length
    private let free: Free
    public convenience init(libraryURL: URL) throws {
        guard libraryURL.isFileURL else { throw LivePhotoAdapterError.invalidResources("native core requires a local library URL") }
        try self.init(handle: dlopen(libraryURL.path, RTLD_NOW | RTLD_LOCAL))
    }
    /// For a host that statically links the C core (e.g. the XCFramework).
    /// Its linker must retain/export the llp_* symbols; no file is dlopened.
    public convenience init(linkedCore: Bool) throws {
        guard linkedCore else { throw LivePhotoAdapterError.unavailable("linkedCore must be explicitly enabled") }
        try self.init(handle: dlopen(nil, RTLD_NOW | RTLD_LOCAL))
    }
    private init(handle: UnsafeMutableRawPointer?) throws {
        guard let handle = handle else { throw LivePhotoAdapterError.unavailable("native core library could not be loaded") }
        func symbol<T>(_ name: String, _ type: T.Type) throws -> T {
            guard let address = dlsym(handle, name) else { throw LivePhotoAdapterError.unavailable("missing native symbol: \(name)") }
            return unsafeBitCast(address, to: type)
        }
        do {
            let version = try symbol("llp_api_version", Version.self)
            guard version() == 1 else { throw LivePhotoAdapterError.unavailable("native core ABI version") }
            process = try symbol("llp_process", Process.self)
            responseBytes = try symbol("llp_response_data", Bytes.self)
            responseLength = try symbol("llp_response_len", Length.self)
            free = try symbol("llp_response_free", Free.self)
            library = handle
        } catch { dlclose(handle); throw error }
    }
    deinit { dlclose(library) }
    /// Data spans remain borrowed only within this synchronous C call. Rust owns
    /// the opaque response; Swift copies it before the matching free in defer.
    /// This operation does not write or upload caller files.
    public func execute(requestJSON: Data, primary: Data = Data(), movie: Data = Data()) throws -> CoreOperationResult {
        guard requestJSON.count <= 1024*1024, primary.count <= 128*1024*1024, movie.count <= 128*1024*1024 - primary.count else { throw LivePhotoAdapterError.inputLimit }
        let response = requestJSON.withUnsafeBytes { request in
            primary.withUnsafeBytes { image in
                movie.withUnsafeBytes { video in
                    process(request.baseAddress?.assumingMemoryBound(to: UInt8.self), UInt(request.count), image.baseAddress?.assumingMemoryBound(to: UInt8.self), UInt(image.count), video.baseAddress?.assumingMemoryBound(to: UInt8.self), UInt(video.count))
                }
            }
        }
        guard let response = response else { throw LivePhotoAdapterError.nativeFailure("native core returned no response") }
        defer { free(response) }
        let length = responseLength(UnsafeRawPointer(response))
        guard length <= 320*1024*1024, let bytes = responseBytes(UnsafeRawPointer(response)) else { throw LivePhotoAdapterError.inputLimit }
        return try CoreOperationResult(envelope: Data(bytes: bytes, count: Int(length)))
    }
    public func inspect(primary: Data, movie: Data? = nil, declaredParts: Bool = false, photoTime: ExactTime? = nil) throws -> CoreOperationResult {
        var request: [String: Any] = ["action": "parse"]
        if movie != nil { request["inputKind"] = declaredParts ? "parts" : "apple" }
        if let time = photoTime { request["time"] = ["value": time.value, "timescale": time.timescale] }
        return try execute(requestJSON: JSONSerialization.data(withJSONObject: request), primary: primary, movie: movie ?? Data())
    }
    @discardableResult public func inspectAsync(primary: Data, movie: Data? = nil, queue: DispatchQueue = .global(qos: .userInitiated), progress: @escaping (OperationProgress) -> Void = { _ in }, completion: @escaping (Result<CoreOperationResult, Error>) -> Void) -> NativeOperation<CoreOperationResult> {
        let op = NativeOperation(progress: progress, completion: completion)
        queue.async { [self] in
            guard op.state == .running else { return }
            op.report(.init(.loading))
            do { op.finish(.success(try inspect(primary: primary, movie: movie))) }
            catch { op.finish(.failure(error)) }
        }
        return op
    }
}
#endif
