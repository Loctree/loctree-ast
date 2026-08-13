import SwiftUI

/// Keeps `RecoveryStore` and both of its entry methods reachable from the
/// `@main` root, so the only unreferenced declaration in the fixture is the
/// deliberate `dormantRecoveryHook` control.
@main
struct FindAgreementApp: App {
    let store = RecoveryStore()

    var body: some Scene {
        WindowGroup {
            Button("purge") {
                store.purge(documentID: "current")
                store.reset()
            }
        }
    }
}
