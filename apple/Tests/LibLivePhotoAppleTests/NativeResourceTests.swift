#if canImport(Photos)
import XCTest
import Photos
@testable import LibLivePhotoApple
final class NativeResourceTests: XCTestCase {
    /// This uses only project-generated local files. It does not access or
    /// mutate a user's Photos library and needs no library add/read permission.
    func testGeneratedLocalPairActuallyLoadsAndExportsOriginalResources() throws {
        let still = Bundle.module.url(forResource: "apple-still", withExtension: "jpg", subdirectory: "Fixtures")!
        let movie = Bundle.module.url(forResource: "apple-paired", withExtension: "mov", subdirectory: "Fixtures")!
        let loaded = expectation(description: "native valid-pair load")
        var loadResult: Result<PHLivePhoto, Error>?
        let operation = PhotoKitAdapter.load(still: still, pairedVideo: movie) { loadResult = $0; loaded.fulfill() }
        wait(for: [loaded], timeout: 20)
        XCTAssertNotEqual(operation.state, .running)
        let livePhoto = try XCTUnwrap(loadResult).get()
        let resources = PhotoKitAdapter.resources(for: livePhoto)
        XCTAssertFalse(resources.isEmpty)
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        for (type, original) in [(PHAssetResourceType.photo, still), (.pairedVideo, movie)] {
            let resource = try XCTUnwrap(resources.first { $0.type == type })
            let exported = expectation(description: "native resource export \(type.rawValue)")
            let output = directory.appendingPathComponent("resource-\(type.rawValue)")
            var exportResult: Result<URL, Error>?
            let exportOperation = PhotoKitAdapter.export(resource, to: output) { exportResult = $0; exported.fulfill() }
            wait(for: [exported], timeout: 20)
            XCTAssertEqual(exportOperation.state, .completed)
            let url = try XCTUnwrap(exportResult).get()
            XCTAssertEqual(try Data(contentsOf: url), try Data(contentsOf: original))
        }
    }
}
#endif
