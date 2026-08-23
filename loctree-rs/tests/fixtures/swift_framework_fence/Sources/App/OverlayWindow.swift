import AppKit

/// The type the framework owns. `plainDormantHelper` is a negative control:
/// ordinary app code with no caller anywhere, so it must stay high-confidence
/// dead. A fence that also silences this one is over-fencing, not fencing.
final class OverlayWindow: NSWindow {
    func plainDormantHelper() {}
}
