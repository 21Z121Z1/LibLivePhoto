import Foundation

public enum LivePhotoAdapterError: Error, Equatable {
    case cancelled
    case unavailable(String)
    case invalidTime
    case invalidResources(String)
    case inputLimit
    case nativeFailure(String)
}
public struct ExactTime: Codable, Equatable {
    public let value: String
    public let timescale: UInt32
    public init(value: Int64, timescale: UInt32) throws {
        guard timescale > 0 else { throw LivePhotoAdapterError.invalidTime }
        self.value = String(value); self.timescale = timescale
    }
    enum CodingKeys: String, CodingKey { case value, timescale }
    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let v = try c.decode(String.self, forKey: .value)
        let scale = try c.decode(UInt32.self, forKey: .timescale)
        guard let integer = Int64(v), String(integer) == v else { throw LivePhotoAdapterError.invalidTime }
        try self.init(value: integer, timescale: scale)
    }
}
/// Versioned subset of the Rust bridge manifest. Unknown fields remain in the
/// original manifest bytes held by the caller; decoding this view is not remux.
public struct InspectionManifest: Decodable {
    public let schemaVersion: Int
    public let format: String
    public let photoTime: ExactTime?
    public let pairIdentifier: String?
    public let resources: [Resource]
    public let tracks: [Track]
    public struct Resource: Decodable {
        public let id: Int; public let role: String; public let mime: String
        public let sourceIndex: Int; public let extents: [Extent]
    }
    public struct Extent: Decodable { public let start: String; public let end: String }
    public struct Track: Decodable {
        public let id: UInt32; public let kind: String; public let timescale: UInt32
        public let duration: String; public let frameTiming: Timing?
    }
    public struct Timing: Decodable { public let status: String; public let frames: [Frame]?; public let reason: String? }
    public struct Frame: Decodable { public let sampleIndex: Int; public let time: ExactTime }
    public static func decode(_ data: Data) throws -> Self {
        let m = try JSONDecoder().decode(Self.self, from: data)
        guard m.schemaVersion == 1 else { throw LivePhotoAdapterError.unavailable("manifest schema version") }
        return m
    }
}
public enum Capability: Equatable { case available; case unavailable(String) }
public enum NativeCapabilities {
    public static var photoKit: Capability {
        #if canImport(Photos)
        return .available
        #else
        return .unavailable("PhotoKit requires an Apple platform")
        #endif
    }
    /// A host-owned session, device configuration and capture delegates are
    /// required. The portable core and the browser do not implement capture.
    public static let capture = Capability.unavailable("requires host AVCaptureSession and AVCapturePhotoCaptureDelegate")
    public static let privatePhotosEffects = Capability.unavailable("no public equivalent promised for private Photos effects")
}

public struct OperationProgress: Equatable {
    public enum Stage: String { case loading, exporting, preparing, editing, committing, complete }
    public let stage: Stage
    /// Native reported fraction, or nil for indeterminate work. Never simulated.
    public let fraction: Double?
    public init(_ stage: Stage, fraction: Double? = nil) { self.stage = stage; self.fraction = fraction }
}
/// Completion occurs once. Callbacks run on the native callback's queue; UI
/// clients dispatch to their own actor/queue. Cancellation is terminal; stale
/// native results are ignored. Photo-library commits cannot be cancelled.
public final class NativeOperation<Value> {
    public enum State: Equatable { case running, committing, completed, failed, cancelled }
    private let lock = NSLock()
    private var current: State = .running
    private var cancelAction: (() -> Void)?
    private let onProgress: (OperationProgress) -> Void
    private let completion: (Result<Value, Error>) -> Void
    public init(progress: @escaping (OperationProgress) -> Void = { _ in }, completion: @escaping (Result<Value, Error>) -> Void) { self.onProgress = progress; self.completion = completion }
    public var state: State { lock.lock(); defer { lock.unlock() }; return current }
    public func installCancellation(_ action: @escaping () -> Void) {
        lock.lock(); let cancelled = current == .cancelled
        if current == .running { cancelAction = action }; lock.unlock()
        if cancelled { action() }
    }
    @discardableResult public func cancel() -> Bool {
        lock.lock(); guard current == .running else { lock.unlock(); return false }
        current = .cancelled; let action = cancelAction; cancelAction = nil; lock.unlock()
        action?(); completion(.failure(LivePhotoAdapterError.cancelled)); return true
    }
    @discardableResult public func beginCommit() -> Bool {
        lock.lock(); guard current == .running else { lock.unlock(); return false }
        current = .committing; cancelAction = nil; lock.unlock(); onProgress(.init(.committing)); return true
    }
    public func report(_ progress: OperationProgress) {
        lock.lock(); let active = current == .running || current == .committing; lock.unlock()
        if active { onProgress(progress) }
    }
    public func finish(_ result: Result<Value, Error>) {
        lock.lock(); guard current == .running || current == .committing else { lock.unlock(); return }
        if case .success = result { current = .completed } else { current = .failed }
        cancelAction = nil; lock.unlock(); completion(result)
    }
}
