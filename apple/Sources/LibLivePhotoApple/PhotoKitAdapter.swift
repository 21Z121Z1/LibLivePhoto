#if canImport(Photos)
import Foundation
import Photos
import PhotosUI
#if canImport(AppKit)
import AppKit
#endif
#if canImport(UIKit)
import UIKit
#endif
import CoreGraphics
import CoreMedia

public extension ExactTime {
    func cmTime() throws -> CMTime {
        guard let value = Int64(value), let scale = Int32(exactly: timescale) else { throw LivePhotoAdapterError.invalidTime }
        return CMTime(value: value, timescale: scale)
    }
}
public enum PhotoKitAdapter {
    /// PhotoKit validates pairing using actual resource files; a Rust manifest
    /// is not an authorization token or proof that these URLs still match it.
    @discardableResult public static func load(still: URL, pairedVideo: URL, targetSize: CGSize = .zero, progress: @escaping (OperationProgress) -> Void = { _ in }, completion: @escaping (Result<PHLivePhoto, Error>) -> Void) -> NativeOperation<PHLivePhoto> {
        let op = NativeOperation(progress: progress, completion: completion)
        op.report(.init(.loading))
        let request = PHLivePhoto.request(withResourceFileURLs: [still, pairedVideo], placeholderImage: nil, targetSize: targetSize, contentMode: .aspectFit) { photo, info in
            if info[PHLivePhotoInfoCancelledKey] as? Bool == true { op.finish(.failure(LivePhotoAdapterError.cancelled)); return }
            if let error = info[PHLivePhotoInfoErrorKey] as? Error { op.finish(.failure(error)); return }
            if info[PHLivePhotoInfoIsDegradedKey] as? Bool == true { return }
            if let photo = photo { op.finish(.success(photo)) }
            else { op.finish(.failure(LivePhotoAdapterError.invalidResources("PhotoKit did not produce a Live Photo"))) }
        }
        op.installCancellation { PHLivePhoto.cancelRequest(withRequestID: request) }
        return op
    }
    public static func resources(for asset: PHAsset) -> [PHAssetResource] { PHAssetResource.assetResources(for: asset) }
    public static func resources(for livePhoto: PHLivePhoto) -> [PHAssetResource] { PHAssetResource.assetResources(for: livePhoto) }

    /// Export one chosen resource, preserving its bytes. The caller explicitly
    /// selects photo/pairedVideo/fullSizePairedVideo/adjustment resources from
    /// the complete inventory. No first-video heuristic or filename pairing.
    /// Network access is opt-in. A new output path is required.
    @discardableResult public static func export(_ resource: PHAssetResource, to destination: URL, networkAccessAllowed: Bool = false, maxBytes: Int = 128 * 1024 * 1024, progress: @escaping (OperationProgress) -> Void = { _ in }, completion: @escaping (Result<URL, Error>) -> Void) -> NativeOperation<URL> {
        let op = NativeOperation(progress: progress, completion: completion)
        guard maxBytes > 0 else { op.finish(.failure(LivePhotoAdapterError.inputLimit)); return op }
        let options = PHAssetResourceRequestOptions(); options.isNetworkAccessAllowed = networkAccessAllowed
        options.progressHandler = { fraction in op.report(.init(.exporting, fraction: fraction)) }
        let lock = NSLock(); var bytes = Data(); var failure: Error?
        op.report(.init(.exporting))
        let manager = PHAssetResourceManager.default()
        let request = manager.requestData(for: resource, options: options, dataReceivedHandler: { chunk in
            lock.lock(); defer { lock.unlock() }
            guard op.state == .running, failure == nil else { return }
            guard chunk.count <= maxBytes - bytes.count else { failure = LivePhotoAdapterError.inputLimit; bytes.removeAll(); return }
            bytes.append(chunk)
        }, completionHandler: { error in
            lock.lock(); defer { lock.unlock() }
            guard op.state == .running else { return }
            do {
                if let error = error ?? failure { throw error }
                // Publication cannot overwrite an existing file. An I/O error may
                // leave a partial new output; originals are never overwritten.
                guard op.beginCommit() else { return }
                try bytes.write(to: destination, options: .withoutOverwriting)
                bytes.removeAll(); op.finish(.success(destination))
            } catch { bytes.removeAll(); op.finish(.failure(error)) }
        })
        op.installCancellation { manager.cancelDataRequest(request); lock.lock(); bytes.removeAll(); lock.unlock() }
        return op
    }
    /// Call only after the host obtains Photo Library add permission. PhotoKit
    /// commits are atomic and non-cancellable once submitted. Originals are
    /// copied, never moved. Completion is PhotoKit acceptance, not Gallery UX.
    @discardableResult public static func importPair(still: URL, pairedVideo: URL, progress: @escaping (OperationProgress) -> Void = { _ in }, completion: @escaping (Result<String, Error>) -> Void) -> NativeOperation<String> {
        let op = NativeOperation(progress: progress, completion: completion)
        guard still.isFileURL, pairedVideo.isFileURL else { op.finish(.failure(LivePhotoAdapterError.invalidResources("local file URLs required"))); return op }
        guard op.beginCommit() else { return op }
        var identifier: String?
        PHPhotoLibrary.shared().performChanges({
            let request = PHAssetCreationRequest.forAsset()
            let photoOptions = PHAssetResourceCreationOptions(); photoOptions.shouldMoveFile = false
            let videoOptions = PHAssetResourceCreationOptions(); videoOptions.shouldMoveFile = false
            request.addResource(with: .photo, fileURL: still, options: photoOptions)
            request.addResource(with: .pairedVideo, fileURL: pairedVideo, options: videoOptions)
            identifier = request.placeholderForCreatedAsset?.localIdentifier
        }) { success, error in
            if success, let identifier = identifier { op.finish(.success(identifier)) }
            else { op.finish(.failure(error ?? LivePhotoAdapterError.nativeFailure("PhotoKit import failed"))) }
        }
        return op
    }
}
#endif
