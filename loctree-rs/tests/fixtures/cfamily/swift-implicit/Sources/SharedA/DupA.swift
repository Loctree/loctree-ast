// Target SharedA — one of two targets in this repo declaring `SharedThing`
// (valid Swift across modules). Bare-name usage of `SharedThing` cannot be
// attributed to either file with confidence from a flat file scan, so no
// implicit edge may be created for it.

import Foundation

struct SharedThing {
    let a: Int = 1
}
