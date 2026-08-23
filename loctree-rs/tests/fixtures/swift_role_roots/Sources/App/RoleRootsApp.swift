import SwiftUI

/// The `@main` owner: a graph root by role, never an orphan to review.
@main
struct RoleRootsApp: App {
    var body: some Scene {
        WindowGroup {
            Text(RoleRootsHelper.roleRootsGreeting())
        }
    }
}
