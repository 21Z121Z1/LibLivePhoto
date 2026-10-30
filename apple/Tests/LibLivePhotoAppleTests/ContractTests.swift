import XCTest
@testable import LibLivePhotoApple
final class ContractTests: XCTestCase {
    func testExactIntegerBeyondJavaScriptPrecision() throws {
        let t = try JSONDecoder().decode(ExactTime.self, from: Data(#"{"value":"9007199254740993","timescale":3}"#.utf8))
        XCTAssertEqual(t.value, "9007199254740993"); XCTAssertEqual(t.timescale, 3)
        XCTAssertThrowsError(try JSONDecoder().decode(ExactTime.self, from: Data(#"{"value":"1","timescale":0}"#.utf8)))
        XCTAssertThrowsError(try JSONDecoder().decode(ExactTime.self, from: Data(#"{"value":"9223372036854775808","timescale":3}"#.utf8)))
    }
    func testCancellationBeforeIdentifierAndLateResultIsTerminal() {
        var results = 0; var cancelled = 0
        let op = NativeOperation<Int> { _ in results += 1 }
        XCTAssertTrue(op.cancel()); op.installCancellation { cancelled += 1 }; op.finish(.success(4))
        XCTAssertFalse(op.cancel()); XCTAssertEqual(results, 1); XCTAssertEqual(cancelled, 1); XCTAssertEqual(op.state, .cancelled)
    }
    func testCommitCannotClaimCancellation() {
        var value = 0
        let op = NativeOperation<Int> { if case let .success(v) = $0 { value = v } }
        XCTAssertTrue(op.beginCommit()); XCTAssertFalse(op.cancel()); op.finish(.success(7)); op.finish(.success(9))
        XCTAssertEqual(value, 7); XCTAssertEqual(op.state, .completed)
    }
    func testUnavailableIsExplicitAndCaptureNeedsLocalDestination() {
        XCTAssertNotEqual(NativeCapabilities.capture, .available)
        XCTAssertNotEqual(NativeCapabilities.privatePhotosEffects, .available)
        XCTAssertThrowsError(try LiveCaptureRequest(movieOutputURL: URL(string: "https://example.com/a.mov")!))
    }
    func testRealRustManifestDecodesExactFrameTiming() throws {
        let url = Bundle.module.url(forResource: "apple-manifest", withExtension: "json", subdirectory: "Fixtures")!
        let manifest = try InspectionManifest.decode(Data(contentsOf: url))
        XCTAssertEqual(manifest.format, "AppleLivePhoto")
        XCTAssertEqual(manifest.photoTime, try ExactTime(value: 1625, timescale: 2000))
        let frames = manifest.tracks.first { $0.kind == "Video" }!.frameTiming!.frames!
        XCTAssertEqual(frames.count, 48)
        let frame = frames[13]
        XCTAssertEqual(Int64(frame.time.value)! * 16, Int64(frame.time.timescale) * 13)
    }
    #if canImport(Photos)
    func testNativeInvalidPairReturnsError() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let still = directory.appendingPathComponent("invalid.jpg"), movie = directory.appendingPathComponent("invalid.mov")
        try Data("invalid".utf8).write(to: still); try Data("invalid".utf8).write(to: movie)
        let done = expectation(description: "PhotoKit rejects invalid local pair")
        let operation = PhotoKitAdapter.load(still: still, pairedVideo: movie) { result in
            if case .success = result { XCTFail("invalid pair was accepted") }
            done.fulfill()
        }
        wait(for: [done], timeout: 15); XCTAssertNotEqual(operation.state, .running)
    }
    func testNativeSDKCapabilityIsCompiled() { XCTAssertEqual(NativeCapabilities.photoKit, .available) }
    #endif
}
