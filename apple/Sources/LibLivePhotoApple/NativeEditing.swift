#if canImport(Photos)
import Foundation
import Photos
import CoreGraphics

/// Native rendering is owned by Photos. The core preserves resources and facts;
/// this session does not reproduce private Photos effects or AGTM rendering.
public final class NativeEditingSession {
    public let context: PHLivePhotoEditingContext
    public let input: PHContentEditingInput
    private let operationLock = NSLock()
    private var activeOperation: UUID?
    public init(input: PHContentEditingInput) throws {
        guard let context = PHLivePhotoEditingContext(livePhotoEditingInput: input) else { throw LivePhotoAdapterError.invalidResources("content editing input has no editable Live Photo") }
        self.context = context
        self.input = input
    }
    public func setAudioVolume(_ volume: Float) throws {
        guard volume.isFinite, (0...1).contains(volume) else { throw LivePhotoAdapterError.invalidResources("audio volume must be finite and in [0,1]") }
        context.audioVolume = volume
    }
    public func setFrameProcessor(_ processor: PHLivePhotoFrameProcessingBlock?) { context.frameProcessor = processor }
    public func makeOutput(adjustmentData: PHAdjustmentData) -> PHContentEditingOutput {
        let output = PHContentEditingOutput(contentEditingInput: input)
        output.adjustmentData = adjustmentData; return output
    }
    private func begin() -> UUID? {
        operationLock.lock(); defer { operationLock.unlock() }
        guard activeOperation == nil else { return nil }
        let id = UUID(); activeOperation = id; return id
    }
    private func end(_ id: UUID) {
        operationLock.lock(); defer { operationLock.unlock() }
        if activeOperation == id { activeOperation = nil }
    }
    @discardableResult public func preview(targetSize: CGSize, completion: @escaping (Result<PHLivePhoto, Error>) -> Void) -> NativeOperation<PHLivePhoto> {
        let op = NativeOperation<PHLivePhoto>(completion: completion); op.report(.init(.preparing))
        guard let id = begin() else { op.finish(.failure(LivePhotoAdapterError.unavailable("editing context is already preparing or rendering"))); return op }
        context.prepareLivePhotoForPlayback(withTargetSize: targetSize, options: nil) { photo, error in
            self.end(id)
            if let photo = photo { op.finish(.success(photo)) }
            else { op.finish(.failure(error ?? LivePhotoAdapterError.nativeFailure("editing preview failed"))) }
        }
        op.installCancellation { [self] in context.cancel(); end(id) }; return op
    }
    /// Writes a PHContentEditingOutput. Host submits it via PHAssetChangeRequest
    /// separately; saving this output does not mutate the Photo Library asset.
    @discardableResult public func save(to output: PHContentEditingOutput, completion: @escaping (Result<PHContentEditingOutput, Error>) -> Void) -> NativeOperation<PHContentEditingOutput> {
        let op = NativeOperation<PHContentEditingOutput>(completion: completion); op.report(.init(.editing))
        guard output.adjustmentData != nil else { op.finish(.failure(LivePhotoAdapterError.invalidResources("output needs explicit host adjustment data"))); return op }
        guard let id = begin() else { op.finish(.failure(LivePhotoAdapterError.unavailable("editing context is already preparing or rendering"))); return op }
        context.saveLivePhoto(to: output, options: nil) { success, error in
            self.end(id)
            if success { op.finish(.success(output)) }
            else { op.finish(.failure(error ?? LivePhotoAdapterError.nativeFailure("editing output failed"))) }
        }
        op.installCancellation { [self] in context.cancel(); end(id) }; return op
    }
}
#endif
