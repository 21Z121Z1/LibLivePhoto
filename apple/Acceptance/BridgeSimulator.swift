import Foundation

// Compiled together with the real Swift bridge and linked Rust static archive.
// Actual iOS Simulator execution; it never accesses a Photos library.
let core = try RustCoreBridge(linkedCore: true)
let image = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
let movie = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[2]))
let parsed = try core.execute(requestJSON: Data("{\"action\":\"parse\"}".utf8), primary: image)
guard parsed.success, try parsed.inspectionManifest().photoTime == ExactTime(value: 812500, timescale: 1000000) else { fatalError("exact native manifest failed") }
let extracted = try core.execute(requestJSON: Data("{\"action\":\"extract\",\"still\":false,\"video\":true,\"xmp\":false}".utf8), primary: image)
guard extracted.success, extracted.files.count == 1, extracted.files[0].bytes == movie else { fatalError("byte-exact native extraction failed") }
let agtmMovie = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[3]))
let raw = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[4]))
let agtm = try core.execute(requestJSON: Data("{\"action\":\"agtm\",\"videoInput\":true,\"time\":{\"value\":\"1\",\"timescale\":1}}".utf8), primary: agtmMovie)
guard agtm.success, agtm.files.count == 1, agtm.files[0].bytes == raw else { fatalError("native AGTM bridge failed") }
let invalid = try core.execute(requestJSON: Data("{\"action\":\"parse\"}".utf8), primary: Data([1, 2, 3]))
guard !invalid.success else { fatalError("invalid bytes accepted") }
print("IOS_SIMULATOR_SWIFT_RUST passed: linked static C ABI, exact time, original movie, complete AGTM, errors")
