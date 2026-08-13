import SwiftUI
import AppKit

/// One of two independent views. `text`, `body` and `makeNSView` are a stored
/// property and two protocol witnesses — SwiftUI and NSViewRepresentable
/// dictate the names. Nothing here is duplicated logic, and nothing here is
/// an export waiting for an importer.
struct HealthTruthViewA: View, NSViewRepresentable {
    let text: String = "alpha"

    var body: some View {
        Text(text)
    }

    func makeNSView(context: Context) -> NSView {
        let field = NSTextField(labelWithString: text)
        field.alignment = .left
        return field
    }
}
