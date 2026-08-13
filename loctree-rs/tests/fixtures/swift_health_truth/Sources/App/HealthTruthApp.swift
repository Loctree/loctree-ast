import SwiftUI

@main
struct HealthTruthApp: App {
    var body: some Scene {
        WindowGroup {
            VStack {
                Text(HealthTruthHelper.healthTruthGreeting())
                HealthTruthViewA()
                HealthTruthViewB()
            }
        }
    }
}
