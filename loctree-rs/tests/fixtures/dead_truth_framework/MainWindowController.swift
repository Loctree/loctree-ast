import AppKit

/// AppKit-owned delegate surface: AppKit invokes toolbar/table methods
/// dynamically. Identifier scan sees zero refs; protocol-gated credit (W9-B)
/// must keep them off high-confidence dead — including multi-line
/// inheritance clauses common in real AppKit controllers.
final class MainWindowController: NSWindowController,
    NSToolbarDelegate,
    NSTableViewDataSource,
    NSTableViewDelegate
{
    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        []
    }

    func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        []
    }

    func numberOfRows(in tableView: NSTableView) -> Int {
        0
    }

    func tableViewSelectionDidChange(_ notification: Notification) {}
}
