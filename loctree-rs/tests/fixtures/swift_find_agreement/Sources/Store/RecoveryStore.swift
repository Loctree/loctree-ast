import Foundation

/// Empiria (Pensieve): `retireRecoveryAssociation` is an instance method whose
/// call sites are an implicit-`self` call and an explicit-`self` call, while the
/// declaration itself lives in an extension in another file. `loct find
/// --literal` resolves both call sites; the dead-export pass must agree.
final class RecoveryStore {
    let index = RecoveryIndex()

    func purge(documentID: String) {
        retireRecoveryAssociation(for: documentID)
    }

    func reset() {
        self.retireRecoveryAssociation(for: "*")
    }
}
