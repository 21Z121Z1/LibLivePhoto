#if canImport(UIKit) && canImport(PhotosUI)
import UIKit
import Photos
import PhotosUI

@MainActor public final class NativePlayback: NSObject, PHLivePhotoViewDelegate {
    public enum State { case idle, playing(PHLivePhotoViewPlaybackStyle) }
    public let view: PHLivePhotoView
    public private(set) var state: State = .idle
    public var onState: ((State) -> Void)?
    public override init() { view = PHLivePhotoView(frame: .zero); super.init(); view.delegate = self; view.contentMode = .scaleAspectFit }
    public func load(_ photo: PHLivePhoto) { view.stopPlayback(); view.livePhoto = photo; state = .idle; onState?(state) }
    public func play(style: PHLivePhotoViewPlaybackStyle = .full) { view.startPlayback(with: style) }
    public func stop() { view.stopPlayback() }
    public func setMuted(_ muted: Bool) { view.isMuted = muted }
    public var playbackGesture: UIGestureRecognizer { view.playbackGestureRecognizer }
    public func livePhotoView(_ livePhotoView: PHLivePhotoView, willBeginPlaybackWith playbackStyle: PHLivePhotoViewPlaybackStyle) { state = .playing(playbackStyle); onState?(state) }
    public func livePhotoView(_ livePhotoView: PHLivePhotoView, didEndPlaybackWith playbackStyle: PHLivePhotoViewPlaybackStyle) { state = .idle; onState?(state) }
}
#endif
