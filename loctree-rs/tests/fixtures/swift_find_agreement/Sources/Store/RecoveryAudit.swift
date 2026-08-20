import Foundation

/// Live only from the test target. Swift/SPM capitalises that directory
/// (`AppTests/`), so a reachability rule keyed on the lowercase `"/tests/"`
/// substring cannot see these call sites even though `loct find --literal` does.
enum RecoveryAudit {
    static func auditRecoveryAssociation(for documentID: String) -> String {
        documentID
    }
}
