import Foundation

extension RecoveryStore {
    /// Live: two call sites in `RecoveryStore.swift`, one implicit `self`,
    /// one explicit. Declared in a sibling file, as Swift extensions usually are.
    func retireRecoveryAssociation(for documentID: String) {
        index.drop(documentID)
    }

    /// Negative control: no call site anywhere. Any credit that keeps the live
    /// method off the high-confidence list must not also mask this one.
    func dormantRecoveryHook(for documentID: String) {
        _ = documentID
    }
}
