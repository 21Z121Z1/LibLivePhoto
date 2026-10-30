#if canImport(AppKit) && canImport(PhotosUI) && !canImport(UIKit)
import AppKit
import Photos
import PhotosUI

/// The actual macOS system live view. Pixel/HDR/gesture acceptance on a user's
/// display is separate from SDK and native object/lifecycle verification.
@MainActor public final class NativePlayback: NSObject, PHLivePhotoViewDelegate {
    public enum State { case idle, playing(PHLivePhotoViewPlaybackStyle) }
    public let view: PHLivePhotoView
    public private(set) var state: State = .idle
    public var onState: ((State) -> Void)?
    public override init() { view = PHLivePhotoView(frame: .zero); super.init(); view.delegate = self }
    public func load(_ photo: PHLivePhoto) { view.stopPlayback(); view.livePhoto = photo; state = .idle; onState?(state) }
    public func play(style: PHLivePhotoViewPlaybackStyle = .full) { view.startPlayback(with: style) }
    public func stop() { view.stopPlayback() }
    public func setMuted(_ muted: Bool) { view.isMuted = muted }
    /// The macOS SDK has no public playbackGestureRecognizer accessor.
    /// Hosts can attach an AppKit gesture and call play/stop explicitly.
    public static let playbackGestureAccess = Capability.unavailable("macOS PHLivePhotoView has no public playbackGestureRecognizer accessor")
    public func livePhotoView(_ livePhotoView: PHLivePhotoView, willBeginPlaybackWith playbackStyle: PHLivePhotoViewPlaybackStyle) { state = .playing(playbackStyle); onState?(state) }
    public func livePhotoView(_ livePhotoView: PHLivePhotoView, didEndPlaybackWith playbackStyle: PHLivePhotoViewPlaybackStyle) { state = .idle; onState?(state) }
}
#endif
