import SwiftUI

/// A `@main` root, so the fixture is a real app rather than a pile of orphans.
@main
struct FenceApp: App {
    var body: some Scene {
        WindowGroup {
            Text("framework-owned")
        }
    }
}
