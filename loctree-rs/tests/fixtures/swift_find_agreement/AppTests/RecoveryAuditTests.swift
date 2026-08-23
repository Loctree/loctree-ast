import XCTest

final class RecoveryAuditTests: XCTestCase {
    func testAuditKeepsIdentifier() {
        XCTAssertEqual(RecoveryAudit.auditRecoveryAssociation(for: "a"), "a")
        XCTAssertEqual(RecoveryAudit.auditRecoveryAssociation(for: "b"), "b")
    }
}
