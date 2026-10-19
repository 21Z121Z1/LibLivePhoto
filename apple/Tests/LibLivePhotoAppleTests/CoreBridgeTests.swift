import XCTest
@testable import LibLivePhotoApple
final class CoreBridgeTests: XCTestCase {
    func testSyntaxVectorsThroughActualNativeBridge() throws {
        guard let library = ProcessInfo.processInfo.environment["LIBLIVEPHOTO_NATIVE_LIBRARY"],
              let fixtures = ProcessInfo.processInfo.environment["LIBLIVEPHOTO_SYNTAX_FIXTURES"] else {
            throw XCTSkip("requires built Rust library and syntax fixtures; CI sets both mandatory paths")
        }
        let core = try RustCoreBridge(libraryURL: URL(fileURLWithPath: library))
        XCTAssertEqual(try runSyntaxContracts(core: core, directory: URL(fileURLWithPath: fixtures), label: "macos-swift-c-abi"), 42)
    }
    func testActuallyLoadsRustAndParsesExactResourcesAndErrors() throws {
        guard let path = ProcessInfo.processInfo.environment["LIBLIVEPHOTO_NATIVE_LIBRARY"] else { throw XCTSkip("requires the built native Rust library; CI sets this mandatory path") }
        let bridge = try RustCoreBridge(libraryURL: URL(fileURLWithPath: path))
        let source = try Data(contentsOf: Bundle.module.url(forResource: "motion-photo", withExtension: "jpg", subdirectory: "Fixtures")!)
        let result = try bridge.inspect(primary: source)
        XCTAssertTrue(result.success)
        let manifest = try result.inspectionManifest()
        XCTAssertEqual(manifest.format, "AndroidMotionPhoto")
        XCTAssertEqual(manifest.photoTime, try ExactTime(value: 812500, timescale: 1000000))
        XCTAssertEqual(manifest.tracks.first { $0.kind == "Video" }?.frameTiming?.frames?.count, 48)
        let extracted = try bridge.execute(requestJSON: Data(#"{"action":"extract","still":false,"video":true,"xmp":false}"#.utf8), primary: source)
        XCTAssertTrue(extracted.success); XCTAssertEqual(extracted.files.count, 1); XCTAssertEqual(extracted.files[0].mime, "video/mp4")
        let bad = try bridge.inspect(primary: Data("invalid".utf8)); XCTAssertFalse(bad.success)
        XCTAssertThrowsError(try RustCoreBridge(libraryURL: URL(string: "https://example.com/native.dylib")!))
    }
    func testCancellationSuppressesNativeWorkBeforeScheduledPhase() throws {
        guard let path = ProcessInfo.processInfo.environment["LIBLIVEPHOTO_NATIVE_LIBRARY"] else { throw XCTSkip("requires built native Rust library") }
        let bridge = try RustCoreBridge(libraryURL: URL(fileURLWithPath: path)); let queue = DispatchQueue(label: "core-bridge-cancel")
        queue.suspend(); defer { queue.resume() }
        let done = expectation(description: "cancellation completion")
        let operation = bridge.inspectAsync(primary: Data(), queue: queue) { result in
            if case .success = result { XCTFail("cancelled native work returned success") }; done.fulfill()
        }
        XCTAssertTrue(operation.cancel()); wait(for: [done], timeout: 2); XCTAssertEqual(operation.state, .cancelled)
    }
}
