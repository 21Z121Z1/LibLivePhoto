#if canImport(AppKit) && canImport(PhotosUI) && !canImport(UIKit)
import AppKit
import PhotosUI
import XCTest
@testable import LibLivePhotoApple
final class PlaybackObjectTests: XCTestCase {
    @MainActor func testActualSystemViewMuteGestureAndLifecycleObjects() async {
        let playback = NativePlayback()
        XCTAssertTrue(playback.view.delegate === playback)
        playback.setMuted(true)
        XCTAssertTrue(playback.view.isMuted)
        playback.setMuted(false)
        XCTAssertFalse(playback.view.isMuted)
        XCTAssertEqual(NativePlayback.playbackGestureAccess, .unavailable("macOS PHLivePhotoView has no public playbackGestureRecognizer accessor"))
        playback.stop()
        if case .idle = playback.state {} else { XCTFail("new view must be idle") }
    }
}
#endif
