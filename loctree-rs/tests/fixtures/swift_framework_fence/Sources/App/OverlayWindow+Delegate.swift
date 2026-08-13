import AppKit

/// The three syntactic shapes `c69205db` fenced, declared the way AppKit code
/// usually declares them — in an extension, in a sibling file. None of them has
/// a Swift call site anywhere, and none ever will: the framework reaches them by
/// selector, by protocol witness, or from the superclass. "No references" is
/// what a *correct* one of these looks like.
///
/// `loadUnusedThing` sits in the same extension as the second negative control:
/// it is ordinary app code and must remain a delete candidate, which proves the
/// fence discriminates by shape rather than by file.
extension OverlayWindow {
    /// Delegate-callback prefix (`window` + uppercase).
    func windowWillClose(_ notification: Notification) {}

    /// Exposed to the ObjC runtime precisely so a selector can reach it.
    @objc func handleCloseTap(_ sender: Any) {}

    /// Dispatched by the superclass.
    override func awakeFromNib() {}

    /// Negative control: no caller, no framework shape.
    func loadUnusedThing() {}
}
