import Foundation
import CryptoKit
#if canImport(LibLivePhotoApple)
import LibLivePhotoApple
#endif

// Test-only acceptance shared by XCTest and the actual iOS Simulator process.
// Every operation calls the maintained Swift -> C ABI -> Rust bridge.
func runSyntaxContracts(core: RustCoreBridge, directory: URL, label: String, expectedCount: Int = 42) throws -> Int {
    func read(_ name: String) throws -> Data { try Data(contentsOf: directory.appendingPathComponent(name)) }
    func require(_ condition: Bool, _ message: String) throws {
        if !condition { throw NSError(domain: "SyntaxContracts", code: 1, userInfo: [NSLocalizedDescriptionKey: message]) }
    }
    let manifestBytes = try read("cases.json")
    let manifest = try JSONSerialization.jsonObject(with: manifestBytes) as! [String: Any]
    let suites = [42: "syntax", 13: "vendor", 20: "vendor-selection"]
    guard let suite = suites[expectedCount] else { throw NSError(domain: "SyntaxContracts", code: 2, userInfo: [NSLocalizedDescriptionKey: "unknown suite"]) }
    let locks = try JSONSerialization.jsonObject(with: Data(contentsOf: directory.deletingLastPathComponent().appendingPathComponent("contract-suites.json"))) as! [String: [String: Any]]
    let lock = locks[suite]!
    let manifestDigest = SHA256.hash(data: manifestBytes).map { String(format: "%02x", $0) }.joined()
    try require(manifestDigest == (lock["manifest_sha256"] as! String), "locked manifest \(suite)")
    let ids = (manifest["cases"] as! [[String: Any]]).map { $0["id"] as! String }
    try require(ids == (lock["case_ids"] as! [String]) && ids.count == expectedCount && (lock["count"] as! Int) == expectedCount, "complete ordered IDs \(suite)")
    for (name, expected) in manifest["sha256"] as! [String: String] {
        let digest = SHA256.hash(data: try read(name)).map { String(format: "%02x", $0) }.joined()
        try require(digest == expected, "fixture hash \(name)")
    }
    var previous: [String: CoreOperationResult] = [:]
    func data(_ source: Any?) throws -> Data {
        guard let source = source else { return Data() }
        if let name = source as? String { return try read(name) }
        let reference = source as! [String: Any]
        return previous[reference["case"] as! String]!.files[(reference["file"] as? Int) ?? 0].bytes
    }
    let cases = manifest["cases"] as! [[String: Any]]
    for c in cases {
        let id = c["id"] as! String
        let request = try JSONSerialization.data(withJSONObject: c["request"]!)
        let result = try core.execute(requestJSON: request, primary: data(c["primary"]), movie: data(c["movie"]))
        try require(result.success == (c["success"] as! Bool), "\(id): status \(String(data: result.headerJSON, encoding: .utf8) ?? "invalid JSON")")
        let header = try JSONSerialization.jsonObject(with: result.headerJSON)
        let facts = c["expect"] as! [[String: Any]]
        for fact in facts {
            var value: Any = header
            for key in fact["path"] as! [Any] {
                if let name = key as? String { value = (value as! [String: Any])[name]! }
                else { value = (value as! [Any])[(key as! NSNumber).intValue] }
            }
            let actual = try JSONSerialization.data(withJSONObject: ["v": value], options: [.sortedKeys])
            let expected = try JSONSerialization.data(withJSONObject: ["v": fact["value"]!], options: [.sortedKeys])
            try require(actual == expected, "\(id): field \(fact["path"]!) actual=\(value)")
        }
        let files = (c["files"] as? [[String: Any]]) ?? []
        try require(result.files.count == files.count, "\(id): file count")
        for (file, expected) in zip(result.files, files) {
            try require(file.mime == (expected["mime"] as! String), "\(id): MIME")
            try require(file.bytes == data(expected["fixture"]), "\(id): media byte preservation")
        }
        previous[id] = result
        print("SYNTAX_CONTRACT_CASE \(label) \(id) fields=\(facts.count) files=\(files.count)")
    }
    try require(cases.count == expectedCount, "complete acceptance suite required")
    let digest = SHA256.hash(data: manifestBytes).map { String(format: "%02x", $0) }.joined()
    print("SYNTAX_CONTRACTS_PASSED entry=\(label) cases=\(cases.count) manifest_sha256=\(digest)")
    return cases.count
}
