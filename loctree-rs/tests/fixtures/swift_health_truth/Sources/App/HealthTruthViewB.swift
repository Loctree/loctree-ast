import SwiftUI
import AppKit

/// The second independent view. Same member names as `HealthTruthViewA` —
/// because the frameworks require them — but every body differs. A health
/// score that reads these three shared names as debt is scoring the parser,
/// not the code.
struct HealthTruthViewB: View, NSViewRepresentable {
    let text: String = "beta"

    var body: some View {
        VStack {
            Text(text)
            Divider()
        }
    }

    func makeNSView(context: Context) -> NSView {
        let spinner = NSProgressIndicator()
        spinner.isIndeterminate = true
        spinner.startAnimation(nil)
        return spinner
    }
}
