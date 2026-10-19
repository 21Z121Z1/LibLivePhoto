#if canImport(Photos)
import Foundation
import Photos
import CoreGraphics

/// Native rendering is owned by Photos. The core preserves resources and facts;
/// this session does not reproduce private Photos effects or AGTM rendering.
public final class NativeEditingSession {
    public let context: PHLivePhotoEditingContext
    public init(input: PHContentEditingInput) throws {
        guard let context = PHLivePhotoEditingContext(livePhotoEditingInput: input) else { throw LivePhotoAdapterError.invalidResources("content editing input has no editable Live Photo") }
        self.context = context
    }
    public func setAudioVolume(_ volume: Float) { context.audioVolume = volume }
    public func setFrameProcessor(_ processor: PHLivePhotoFrameProcessingBlock?) { context.frameProcessor = processor }
    @discardableResult public func preview(targetSize: CGSize, completion: @escaping (Result<PHLivePhoto, Error>) -> Void) -> NativeOperation<PHLivePhoto> {
        let op = NativeOperation<PHLivePhoto>(completion: completion); op.report(.init(.preparing))
        context.prepareLivePhotoForPlayback(withTargetSize: targetSize, options: nil) { photo, error in
            if let photo = photo { op.finish(.success(photo)) }
            else { op.finish(.failure(error ?? LivePhotoAdapterError.nativeFailure("editing preview failed"))) }
        }
        op.installCancellation { [context] in context.cancel() }; return op
    }
    /// Writes a PHContentEditingOutput. Host submits it via PHAssetChangeRequest
    /// separately; saving this output does not mutate the Photo Library asset.
    @discardableResult public func save(to output: PHContentEditingOutput, completion: @escaping (Result<PHContentEditingOutput, Error>) -> Void) -> NativeOperation<PHContentEditingOutput> {
        let op = NativeOperation<PHContentEditingOutput>(completion: completion); op.report(.init(.editing))
        context.saveLivePhoto(to: output, options: nil) { success, error in
            if success { op.finish(.success(output)) }
            else { op.finish(.failure(error ?? LivePhotoAdapterError.nativeFailure("editing output failed"))) }
        }
        op.installCancellation { [context] in context.cancel() }; return op
    }
}
#endif
