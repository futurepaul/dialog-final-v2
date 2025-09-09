import SwiftUI

@main
struct DialogApp: App {
    init() {
        // Silence noisy simulator logs about eligibility.plist by creating the expected path in the app container.
        // This is a best-effort hack for Simulator-only noise; harmless on device.
        let home = NSHomeDirectory()
        let dir = URL(fileURLWithPath: home).appendingPathComponent("private/var/db/eligibilityd", isDirectory: true)
        let file = dir.appendingPathComponent("eligibility.plist", conformingTo: .propertyList)
        do {
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            if !FileManager.default.fileExists(atPath: file.path) {
                let empty: [String: Any] = [:]
                let data = try PropertyListSerialization.data(fromPropertyList: empty, format: .xml, options: 0)
                try data.write(to: file, options: .atomic)
            }
        } catch {
            // Ignore failures; this is a non-critical simulator cleanup.
            print("[swift] eligibility plist setup failed:", error.localizedDescription)
        }
    }
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
