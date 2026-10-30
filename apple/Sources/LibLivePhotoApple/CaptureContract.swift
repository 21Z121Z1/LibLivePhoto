import Foundation
/// Contract for a host-owned AVFoundation capture adapter. No camera session is
/// created by the library. Both outputs, errors and capture identity must be
/// routed from the same AVCapturePhotoCaptureDelegate request.
public struct LiveCaptureRequest {
    public let movieOutputURL: URL
    public init(movieOutputURL: URL) throws {
        guard movieOutputURL.isFileURL else { throw LivePhotoAdapterError.invalidResources("capture requires a local movie output URL") }
        self.movieOutputURL = movieOutputURL
    }
}
#if canImport(AVFoundation) && os(iOS)
import AVFoundation
public enum CaptureContract {
    /// The host configures/owns the session and keeps its delegate alive. Capture
    /// callbacks must include photo data, didFinishRecordingLivePhotoMovie and
    /// didFinishProcessingLivePhotoToMovieFileAt before committing a pair.
    public static func settings(for request: LiveCaptureRequest, output: AVCapturePhotoOutput) throws -> AVCapturePhotoSettings {
        guard output.isLivePhotoCaptureSupported, output.isLivePhotoCaptureEnabled else { throw LivePhotoAdapterError.unavailable("Live Photo capture is disabled or unsupported by this output") }
        let settings = AVCapturePhotoSettings(); settings.livePhotoMovieFileURL = request.movieOutputURL; return settings
    }
}
#endif
