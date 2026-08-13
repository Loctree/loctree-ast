//! Allocator lifecycle helpers for long-lived editor processes.
//!
//! Workspace scans and snapshot deserialization create large temporary object
//! graphs. Rust drops them correctly, but macOS malloc can retain the empty
//! arenas for the lifetime of the LSP process. Ask the allocator to return
//! those unused pages after a scan/load boundary; live allocations are never
//! touched.

#[cfg(target_os = "macos")]
pub(crate) fn release_unused_allocator_memory() -> usize {
    unsafe extern "C" {
        fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
    }

    // SAFETY: Apple documents a null zone as all malloc zones and a zero goal
    // as maximal pressure relief. Only allocator-owned free pages are released.
    unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn release_unused_allocator_memory() -> usize {
    0
}
