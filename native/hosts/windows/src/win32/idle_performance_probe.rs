//! Fixed direct-Windows static-window idle-performance diagnostic.
//!
//! The probe measures only this host process after its first static window
//! paint. It takes no product or application input and uses one delayed timer,
//! so measuring idle cost does not itself create an idle polling workload.

use std::{
    collections::BTreeMap,
    io,
    sync::{
        Mutex, OnceLock,
        mpsc::{SyncSender, TryRecvError, sync_channel},
    },
    time::{Duration, Instant},
};

use super::{
    DestroyWindow, Hwnd, KillTimer, SetTimer, Uint, WindowDefinition,
    launch::run_windows_after_shown, stats,
};

const TIMER_ID: usize = 0xA203;
const SAMPLE_MILLIS: Uint = 30_000;
const PERCENT_MILLIONTHS: u64 = 1_000_000;

static PROBES: OnceLock<Mutex<BTreeMap<Hwnd, Probe>>> = OnceLock::new();

struct Probe {
    started: Instant,
    started_cpu_100ns: u64,
    sender: SyncSender<Result<IdleReport, ()>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct IdleReport {
    sample_milliseconds: u64,
    cpu_time_microseconds: u64,
    cpu_percent_millionths: u64,
    working_set_bytes: u64,
    private_bytes: u64,
}

impl IdleReport {
    fn from_sample(started_cpu_100ns: u64, elapsed: Duration) -> Option<Self> {
        let ended_cpu_100ns = stats::process_cpu_time_100ns()?;
        let cpu_100ns = ended_cpu_100ns.checked_sub(started_cpu_100ns)?;
        let elapsed_100ns = elapsed.as_nanos().checked_div(100)?;
        if elapsed_100ns == 0 {
            return None;
        }
        let cpu_percent_millionths = u128::from(cpu_100ns)
            .checked_mul(100_000_000)?
            .checked_div(elapsed_100ns)?
            .try_into()
            .ok()?;
        let memory = stats::memory_readings();
        Some(Self {
            sample_milliseconds: elapsed.as_millis().try_into().unwrap_or(u64::MAX),
            cpu_time_microseconds: cpu_100ns / 10,
            cpu_percent_millionths,
            working_set_bytes: memory.working_set_bytes,
            private_bytes: memory.private_bytes,
        })
    }

    fn cpu_percent_parts(self) -> (u64, u64) {
        (
            self.cpu_percent_millionths / PERCENT_MILLIONTHS,
            self.cpu_percent_millionths % PERCENT_MILLIONTHS,
        )
    }
}

/// Runs one fixed 30-second static-window idle measurement.
pub(crate) fn run() -> io::Result<()> {
    let (sender, receiver) = sync_channel(1);
    run_windows_after_shown(
        vec![WindowDefinition::document(
            "Anodrel Idle Performance Report",
            "Static host measurement",
            "This fixed direct Windows diagnostic measures this host's CPU time and memory after one static native window has been shown for 30 seconds. Do not interact with the window; it closes itself and prints one local report. It creates no application session, background monitor, network request, or policy change.",
            760,
            440,
        )],
        None,
        move |windows| attach(windows[0], sender),
    )?;
    match receiver.try_recv() {
        Ok(Ok(report)) => {
            print_report(report);
            Ok(())
        }
        Ok(Err(())) | Err(TryRecvError::Empty | TryRecvError::Disconnected) => Err(
            io::Error::other("Windows idle-performance report did not complete its fixed sample"),
        ),
    }
}

fn attach(window: Hwnd, sender: SyncSender<Result<IdleReport, ()>>) -> io::Result<()> {
    let started_cpu_100ns = stats::process_cpu_time_100ns()
        .ok_or_else(|| io::Error::other("Windows process-time reading unavailable"))?;
    // SAFETY: the fully shown static host window belongs to the current UI
    // thread. One delayed timer is the whole sample schedule.
    let timer = unsafe { SetTimer(window, TIMER_ID, SAMPLE_MILLIS, 0) };
    if timer == 0 {
        return Err(io::Error::other(
            "idle-performance report timer unavailable",
        ));
    }
    let mut probes = lock_probes();
    if probes.contains_key(&window) {
        drop(probes);
        // SAFETY: setup failed for this current-thread window, so the delayed
        // timer must not remain behind without its matching probe state.
        unsafe { KillTimer(window, TIMER_ID) };
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "idle-performance report already attached",
        ));
    }
    probes.insert(
        window,
        Probe {
            started: Instant::now(),
            started_cpu_100ns,
            sender,
        },
    );
    Ok(())
}

/// Services the probe's one delayed timer and reports whether it owned it.
pub(super) fn service_timer(window: Hwnd, timer: usize) -> bool {
    if timer != TIMER_ID {
        return false;
    }
    let Some(probe) = take(window) else {
        return true;
    };
    let report = IdleReport::from_sample(probe.started_cpu_100ns, probe.started.elapsed());
    let _ = probe.sender.try_send(report.ok_or(()));
    // SAFETY: this probe is serviced only by the owning host window's UI thread.
    unsafe { DestroyWindow(window) };
    true
}

/// Fails a probe that another close or shutdown route ended early.
pub(super) fn remove(window: Hwnd) {
    if let Some(probe) = take(window) {
        let _ = probe.sender.try_send(Err(()));
    }
}

fn take(window: Hwnd) -> Option<Probe> {
    let probe = lock_probes().remove(&window);
    if probe.is_some() {
        // SAFETY: stopping an absent timer is a no-op. This path runs on the
        // window's own UI thread from either its timer or destruction message.
        unsafe { KillTimer(window, TIMER_ID) };
    }
    probe
}

fn print_report(report: IdleReport) {
    let (whole_percent, fraction_percent) = report.cpu_percent_parts();
    println!(
        concat!(
            "{{\"benchmark\":\"anodrel.host.idle.v1\",",
            "\"sampleMilliseconds\":{},",
            "\"cpuTimeMicroseconds\":{},\"cpuPercent\":{}.{:06},",
            "\"workingSetBytes\":{},\"privateBytes\":{},",
            "\"scope\":\"single static host window after first paint; no application session, process tree, or input workload\"}}"
        ),
        report.sample_milliseconds,
        report.cpu_time_microseconds,
        whole_percent,
        fraction_percent,
        report.working_set_bytes,
        report.private_bytes,
    );
}

fn lock_probes() -> std::sync::MutexGuard<'static, BTreeMap<Hwnd, Probe>> {
    match PROBES.get_or_init(|| Mutex::new(BTreeMap::new())).lock() {
        Ok(probes) => probes,
        // The window procedure contains panics. Retaining only the fixed sample
        // state lets WM_DESTROY clear its one timer and report a safe failure.
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{IdleReport, PERCENT_MILLIONTHS};

    #[test]
    fn cpu_percentage_keeps_six_fixed_decimal_places_without_float_math() {
        let report = IdleReport {
            sample_milliseconds: 30_000,
            cpu_time_microseconds: 1_000,
            cpu_percent_millionths: PERCENT_MILLIONTHS + 2,
            working_set_bytes: 0,
            private_bytes: 0,
        };
        assert_eq!(report.cpu_percent_parts(), (1, 2));
    }

    #[test]
    fn zero_wall_time_never_becomes_a_cpu_percentage() {
        assert!(IdleReport::from_sample(0, Duration::ZERO).is_none());
    }
}
