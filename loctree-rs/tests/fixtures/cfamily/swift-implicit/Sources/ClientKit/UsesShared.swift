// Valid Swift: imports SharedA and uses `SharedThing` by bare name. But the
// repo also contains SharedB.SharedThing, and loctree's flat file scan cannot
// know which module resolved the name — with two exporters the target is a
// guess, so no implicit edge may be created.

import SharedA

struct SharedClient {
    func make() {
        _ = SharedThing()
    }
}
