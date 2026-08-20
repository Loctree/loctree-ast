import SwiftUI

@main
struct RuntimeFixtureApp: App {
    let cacheRoot = ProcessInfo.processInfo.environment["FIXTURE_CACHE_ROOT"]

    var body: some Scene {
        WindowGroup { Text(cacheRoot ?? "") }
    }
}
