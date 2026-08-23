import SwiftUI

/// Consumes both views by name so neither is dead — the fixture isolates the
/// namesake question and nothing else.
@main
struct NamesakeApp: App {
    var body: some Scene {
        WindowGroup {
            VStack {
                NamesakeViewA()
                NamesakeViewB()
            }
        }
    }
}
