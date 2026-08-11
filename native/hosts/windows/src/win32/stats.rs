//! Process readings the Startup Lab reports about itself.
//!
//! Only measurements the host can take about its own process are exposed. This
//! is diagnostics, not a capability: nothing here reads another process, the
//! filesystem, or any user data.

use std::mem;

use super::Dword;

#[repr(C)]
#[derive(Default)]
struct ProcessMemoryCounters {
    cb: Dword,
    page_fault_count: Dword,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> isize;
    fn K32GetProcessMemoryInfo(
        process: isize,
        counters: *mut ProcessMemoryCounters,
        size: Dword,
    ) -> i32;
}

/// What this process currently costs in memory.
///
/// Both figures, because they answer different questions and only one of them
/// survives a comparison between runtimes:
///
/// - **Working set** is the physical memory resident right now. It moves with
///   system pressure, and it counts pages shared with other processes.
/// - **Private bytes** is committed memory this process cannot share. It is the
///   figure that adds up honestly across a process tree, which is what a
///   multi-process runtime forces a comparison to do.
///
/// See `docs/PERFORMANCE.md` before quoting either one.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct MemoryReadings {
    pub(super) working_set_bytes: u64,
    pub(super) private_bytes: u64,
}

/// Returns this process's memory readings, or zeroes when unavailable.
///
/// A failed query reports zero rather than propagating: a diagnostic reading
/// must never be able to stop the surface from opening.
pub(super) fn memory_readings() -> MemoryReadings {
    let mut counters = ProcessMemoryCounters {
        cb: mem::size_of::<ProcessMemoryCounters>() as Dword,
        ..ProcessMemoryCounters::default()
    };
    // SAFETY: GetCurrentProcess returns a pseudo-handle for this process, and
    // `counters` is writable storage whose declared size matches the struct.
    let queried = unsafe {
        K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            mem::size_of::<ProcessMemoryCounters>() as Dword,
        )
    };
    if queried == 0 {
        MemoryReadings::default()
    } else {
        MemoryReadings {
            working_set_bytes: counters.working_set_size as u64,
            // `PROCESS_MEMORY_COUNTERS.PagefileUsage` is what Task Manager
            // shows as "Commit size" and what perfmon calls Private Bytes.
            private_bytes: counters.pagefile_usage as u64,
        }
    }
}

/// Returns this process's working set in bytes, or `0` when unavailable.
pub(super) fn working_set_bytes() -> u64 {
    memory_readings().working_set_bytes
}

#[cfg(test)]
mod tests {
    use super::{memory_readings, working_set_bytes};

    #[test]
    fn the_working_set_is_a_plausible_size() {
        let bytes = working_set_bytes();
        assert!(bytes > 256 * 1024, "implausibly small working set: {bytes}");
        assert!(bytes < 8 * 1024 * 1024 * 1024, "implausibly large: {bytes}");
    }

    #[test]
    fn both_readings_are_present_and_independently_plausible() {
        // Private bytes and the working set are different measurements, so
        // neither may be reported as the other. They are checked separately
        // rather than against each other: their order is not guaranteed, since
        // committed memory can be paged out and resident memory can be shared.
        let readings = memory_readings();
        assert!(readings.working_set_bytes > 256 * 1024);
        assert!(
            readings.private_bytes > 256 * 1024,
            "implausibly small private bytes: {}",
            readings.private_bytes
        );
        assert!(readings.private_bytes < 8 * 1024 * 1024 * 1024);
    }
}
