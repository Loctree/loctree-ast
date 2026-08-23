import XCTest

/// SPM/Swift convention capitalises the test directory (`AppTests/`, `Tests/`).
/// The orphan rule matches the lowercase `"/tests/"` substring only, so this
/// role root is reported as a file "nobody imports" — it is a test entrypoint,
/// which XCTest discovers by reflection, and must be a root instead.
final class RoleRootsHelperTests: XCTestCase {
    func testGreeting() {
        XCTAssertEqual(RoleRootsHelper.roleRootsGreeting(), "hello")
    }
}
