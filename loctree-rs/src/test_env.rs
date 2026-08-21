//! Process-environment mutation for tests — the one audited `unsafe` site.
//!
//! `std::env::set_var` / `remove_var` are `unsafe` since Rust 2024 because a
//! concurrent `getenv` on another thread is undefined behaviour on POSIX.
//! Tests that steer code through environment variables keep that unsafety
//! in this module instead of repeating an `unsafe` block at every call
//! site, so the audit has one place to look.
//!
//! Contract for callers: the test must be serialised with every other test
//! that reads or writes the same variables (the crate uses `serial_test`
//! groups for this, e.g. `aicx_env`). Nothing here enforces that; it is
//! what makes the single `unsafe` below sound.

use std::ffi::OsStr;

/// Set a process environment variable from a serialised test.
pub(crate) fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    // SAFETY: test-only; callers are serialised per the module contract, so
    // no other thread observes the environment while it changes.
    unsafe { std::env::set_var(key, value) }
}

/// Remove a process environment variable from a serialised test.
pub(crate) fn remove_var<K: AsRef<OsStr>>(key: K) {
    // SAFETY: as for `set_var`.
    unsafe { std::env::remove_var(key) }
}
