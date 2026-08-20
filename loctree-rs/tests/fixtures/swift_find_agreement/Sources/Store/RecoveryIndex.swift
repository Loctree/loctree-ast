import Foundation

/// Live across a file boundary: `drop` is reached through a stored property
/// on `DocumentStore` — the shape the falsification probe already showed working.
final class RecoveryIndex {
    func drop(_ documentID: String) {
        _ = documentID
    }
}
