//! Small library face of `loctree-mcp`.
//!
//! The binary owns the server; this target exists only for the pieces that
//! integration tests import directly. `auth` and `security` deliberately are
//! *not* re-exported here: they are `pub(crate)` to the binary, so mounting
//! them in the lib compiled them a second time with every item unreachable —
//! which is why the module-wide `allow(dead_code)` existed at all.

pub mod extract;

pub use extract::extract_symbol;
