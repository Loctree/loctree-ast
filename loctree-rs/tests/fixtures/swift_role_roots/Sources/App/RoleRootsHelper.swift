/// Ordinary library surface: used by the app and by the test target. It is the
/// only file here that is a non-orphan by edges rather than by role — it exists
/// so the detector still has something real to see.
enum RoleRootsHelper {
    static func roleRootsGreeting() -> String {
        "hello"
    }
}
