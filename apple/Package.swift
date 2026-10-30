// swift-tools-version: 5.9
import PackageDescription
let package = Package(name: "LibLivePhotoApple", platforms: [.macOS(.v13), .iOS(.v16)], products: [.library(name: "LibLivePhotoApple", targets: ["LibLivePhotoApple"])], targets: [.target(name: "LibLivePhotoApple"), .testTarget(name: "LibLivePhotoAppleTests", dependencies: ["LibLivePhotoApple"], resources: [.copy("Fixtures")])])
