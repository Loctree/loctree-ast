import SwiftUI
import AppKit

/// The second independent view. Same member names as `NamesakeViewA` — because
/// the frameworks require them — but every body differs. A twins pass that
/// reports `text` / `body` / `makeNSView` as duplicate groups is naming a
/// collision, not a duplication.
struct NamesakeViewB: View, NSViewRepresentable {
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
