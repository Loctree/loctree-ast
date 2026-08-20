import SwiftUI
import AppKit

/// One of two independent views. `text`, `body` and `makeNSView` are a stored
/// property and two protocol witnesses — the names are dictated by SwiftUI and
/// NSViewRepresentable, not chosen. Nothing here is duplicated logic.
struct NamesakeViewA: View, NSViewRepresentable {
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
