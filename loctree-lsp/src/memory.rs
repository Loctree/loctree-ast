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

/// Current resident set size of this process, in bytes.
///
/// The RSS sentinel needs CURRENT residency, not the `getrusage` peak — after
/// a successful pressure relief the current number falls while the peak never
/// does, and a restart decision taken on the peak would kill a recovered
/// server.
#[cfg(target_os = "macos")]
pub(crate) fn current_rss_bytes() -> Option<u64> {
    // Layout of `mach_task_basic_info` (flavor MACH_TASK_BASIC_INFO = 20).
    #[repr(C)]
    struct MachTaskBasicInfo {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        user_time: [i32; 2],
        system_time: [i32; 2],
        policy: i32,
        suspend_count: i32,
    }
    const MACH_TASK_BASIC_INFO: u32 = 20;
    unsafe extern "C" {
        static mach_task_self_: u32;
        fn task_info(task: u32, flavor: u32, info: *mut i32, count: *mut u32) -> i32;
    }
    let mut info = std::mem::MaybeUninit::<MachTaskBasicInfo>::uninit();
    let mut count = (std::mem::size_of::<MachTaskBasicInfo>() / std::mem::size_of::<i32>()) as u32;
    // SAFETY: out-buffer and count match the advertised flavor's layout;
    // `mach_task_self_` is the calling task's port.
    let kr = unsafe {
        task_info(
            mach_task_self_,
            MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast::<i32>(),
            &mut count,
        )
    };
    if kr != 0 {
        return None;
    }
    // SAFETY: task_info returned KERN_SUCCESS, the struct is initialized.
    Some(unsafe { info.assume_init() }.resident_size)
}

#[cfg(target_os = "linux")]
pub(crate) fn current_rss_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    // SAFETY: sysconf with a valid name has no memory-safety concerns.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    Some(resident_pages.saturating_mul(if page > 0 { page as u64 } else { 4096 }))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) fn current_rss_bytes() -> Option<u64> {
    None
}

/// RSS sentinel thresholds for long-lived editor processes.
///
/// Field evidence (2026-08-19): editor-embedded loctree-lsp instances reached
/// 3.9 GB / 3.0 GB RSS after ~38 h of uptime. The soft limit sheds every
/// rebuildable cache and asks the allocator to return free pages; the hard
/// limit — measured AGAIN after relief — exits the process so the editor's
/// standard LSP restart brings back a fresh server. Either leg can be
/// disabled by setting its env var to 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RssSentinelConfig {
    pub soft_limit_bytes: u64,
    pub hard_limit_bytes: u64,
    pub check_interval_secs: u64,
}

pub(crate) const RSS_SOFT_MB_ENV: &str = "LOCTREE_LSP_RSS_SOFT_MB";
pub(crate) const RSS_HARD_MB_ENV: &str = "LOCTREE_LSP_RSS_HARD_MB";
pub(crate) const RSS_CHECK_SECS_ENV: &str = "LOCTREE_LSP_RSS_CHECK_SECS";
const DEFAULT_RSS_SOFT_MB: u64 = 1536;
const DEFAULT_RSS_HARD_MB: u64 = 3072;
const DEFAULT_RSS_CHECK_SECS: u64 = 60;
const MEBIBYTE: u64 = 1024 * 1024;

impl Default for RssSentinelConfig {
    fn default() -> Self {
        Self::from_env_reader(|_| None)
    }
}

impl RssSentinelConfig {
    pub(crate) fn from_env() -> Self {
        Self::from_env_reader(|name| std::env::var(name).ok())
    }

    fn from_env_reader(read: impl Fn(&str) -> Option<String>) -> Self {
        let mb = |name: &str, default: u64| -> u64 {
            read(name)
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(default)
        };
        Self {
            soft_limit_bytes: mb(RSS_SOFT_MB_ENV, DEFAULT_RSS_SOFT_MB).saturating_mul(MEBIBYTE),
            hard_limit_bytes: mb(RSS_HARD_MB_ENV, DEFAULT_RSS_HARD_MB).saturating_mul(MEBIBYTE),
            check_interval_secs: mb(RSS_CHECK_SECS_ENV, DEFAULT_RSS_CHECK_SECS).max(10),
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.soft_limit_bytes > 0 || self.hard_limit_bytes > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_is_observable_on_supported_platforms() {
        if cfg!(any(target_os = "macos", target_os = "linux")) {
            let rss = current_rss_bytes().expect("RSS readable on macOS/Linux");
            assert!(rss > MEBIBYTE, "a live test process holds >1MiB, got {rss}");
        }
    }

    #[test]
    fn sentinel_config_reads_env_and_disables_on_zero() {
        let cfg = RssSentinelConfig::from_env_reader(|name| match name {
            RSS_SOFT_MB_ENV => Some("0".into()),
            RSS_HARD_MB_ENV => Some("2048".into()),
            RSS_CHECK_SECS_ENV => Some("5".into()),
            _ => None,
        });
        assert_eq!(cfg.soft_limit_bytes, 0);
        assert_eq!(cfg.hard_limit_bytes, 2048 * MEBIBYTE);
        assert_eq!(
            cfg.check_interval_secs, 10,
            "interval clamps to a sane floor"
        );
        assert!(cfg.enabled());

        let off = RssSentinelConfig::from_env_reader(|name| {
            (name == RSS_SOFT_MB_ENV || name == RSS_HARD_MB_ENV).then(|| "0".into())
        });
        assert!(!off.enabled());
    }
}
