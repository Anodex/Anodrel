//! Fixed direct-Windows taskbar-progress diagnostic.
//!
//! This probe deliberately owns no product-update state. It exists to make
//! Explorer's taskbar-button ordering observable without an installed product,
//! certificate, network request, or policy change. See Decision 0201.

use std::{
    collections::BTreeMap,
    io,
    sync::{
        Mutex, OnceLock,
        mpsc::{SyncSender, TryRecvError, sync_channel},
    },
};

use anodrel_windows_taskbar_progress::{TaskbarProgress, set_taskbar_progress};

use super::{
    DestroyWindow, Hwnd, KillTimer, SetTimer, Uint, WindowDefinition,
    launch::run_windows_after_created,
};

const TIMER_ID: usize = 0xA201;
const TIMER_MILLIS: Uint = 150;
const READINESS_TIMEOUT_TICKS: u8 = 40;
const PROGRESS_STEP: u8 = 10;
const PROGRESS_TOTAL: u64 = 100;

static PROBES: OnceLock<Mutex<BTreeMap<Hwnd, Probe>>> = OnceLock::new();

struct Probe {
    state: State,
    taskbar_visible: bool,
    sender: SyncSender<Result<(), ()>>,
}

#[derive(Default)]
struct State {
    taskbar_ready: bool,
    waited_ticks: u8,
    next_percent: u8,
}

#[derive(Debug, Eq, PartialEq)]
enum Next {
    Wait,
    Present(TaskbarProgress),
    ClearAndSucceed,
    Fail,
}

impl State {
    fn taskbar_button_created(&mut self) -> bool {
        if self.taskbar_ready {
            return false;
        }
        self.taskbar_ready = true;
        self.waited_ticks = 0;
        self.next_percent = 0;
        true
    }

    fn taskbar_restarted(&mut self) {
        self.taskbar_ready = false;
        self.waited_ticks = 0;
        self.next_percent = 0;
    }

    fn next(&mut self) -> Next {
        if !self.taskbar_ready {
            self.waited_ticks = self.waited_ticks.saturating_add(1);
            return if self.waited_ticks >= READINESS_TIMEOUT_TICKS {
                Next::Fail
            } else {
                Next::Wait
            };
        }
        if self.next_percent <= PROGRESS_TOTAL as u8 {
            let completed = self.next_percent;
            self.next_percent = self.next_percent.saturating_add(PROGRESS_STEP);
            return Next::Present(TaskbarProgress::Determinate {
                completed: u64::from(completed),
                total: PROGRESS_TOTAL,
            });
        }
        Next::ClearAndSucceed
    }
}

/// Opens the fixed diagnostic and reports success only after the taskbar state
/// has been cleared and the host window has closed.
pub(crate) fn run() -> io::Result<()> {
    let (sender, receiver) = sync_channel(1);
    run_windows_after_created(
        vec![WindowDefinition::document(
            "Anodrel Taskbar Progress Probe",
            "Direct Windows taskbar validation",
            "This fixed host diagnostic waits for Windows to create its taskbar button, then presents activity and 0–100% progress before clearing itself. It takes no product, application, network, installer, or certificate input.",
            760,
            440,
        )],
        None,
        move |windows| attach(windows[0], sender),
    )?;
    match receiver.try_recv() {
        Ok(Ok(())) => {
            println!("Windows taskbar progress probe passed.");
            Ok(())
        }
        Ok(Err(())) | Err(TryRecvError::Empty | TryRecvError::Disconnected) => Err(
            io::Error::other("Windows taskbar progress probe did not complete"),
        ),
    }
}

fn attach(window: Hwnd, sender: SyncSender<Result<(), ()>>) -> io::Result<()> {
    // SAFETY: this newly created host window belongs to the current UI thread.
    let timer = unsafe { SetTimer(window, TIMER_ID, TIMER_MILLIS, 0) };
    if timer == 0 {
        return Err(io::Error::other("taskbar progress probe timer unavailable"));
    }
    let mut probes = lock_probes();
    if probes.contains_key(&window) {
        drop(probes);
        // SAFETY: this window belongs to the current UI thread and its probe
        // state will not retain the timer after this failed setup.
        unsafe { KillTimer(window, TIMER_ID) };
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "taskbar progress probe already attached",
        ));
    }
    probes.insert(
        window,
        Probe {
            state: State::default(),
            taskbar_visible: false,
            sender,
        },
    );
    Ok(())
}

/// Records the exact host window's shell-created taskbar button before the
/// first direct taskbar call.
pub(super) fn taskbar_button_created(window: Hwnd) {
    let should_present = {
        let mut probes = lock_probes();
        probes
            .get_mut(&window)
            .map(|probe| probe.state.taskbar_button_created())
            .unwrap_or(false)
    };
    if should_present {
        present(window, TaskbarProgress::Activity);
    }
}

/// Resets only this diagnostic after Explorer recreates taskbar buttons.
pub(super) fn taskbar_restarted(window: Hwnd) {
    let mut probes = lock_probes();
    if let Some(probe) = probes.get_mut(&window) {
        probe.state.taskbar_restarted();
        probe.taskbar_visible = false;
    }
}

/// Services the diagnostic's fixed UI-thread timer and reports whether its
/// timer ID owned the message.
pub(super) fn service_timer(window: Hwnd, timer: usize) -> bool {
    if timer != TIMER_ID {
        return false;
    }
    let next = {
        let mut probes = lock_probes();
        probes.get_mut(&window).map(|probe| probe.state.next())
    };
    match next {
        Some(Next::Wait) | None => {}
        Some(Next::Fail) => finish(window, Err(()), true),
        Some(Next::Present(progress)) => present(window, progress),
        Some(Next::ClearAndSucceed) => {
            if set_taskbar_progress(window, TaskbarProgress::Clear) {
                finish(window, Ok(()), false);
            } else {
                finish(window, Err(()), true);
            }
        }
    }
    true
}

/// Removes a probe when its host window is destroyed by the user or another
/// host shutdown route.
pub(super) fn remove(window: Hwnd) {
    let probe = take(window);
    if let Some(probe) = probe {
        clear_if_visible(window, probe.taskbar_visible);
        let _ = probe.sender.try_send(Err(()));
    }
}

fn present(window: Hwnd, progress: TaskbarProgress) {
    if set_taskbar_progress(window, progress) {
        let mut probes = lock_probes();
        if let Some(probe) = probes.get_mut(&window) {
            probe.taskbar_visible = true;
        }
    } else {
        finish(window, Err(()), true);
    }
}

fn finish(window: Hwnd, result: Result<(), ()>, clear_taskbar: bool) {
    let probe = take(window);
    if let Some(probe) = probe {
        if clear_taskbar {
            clear_if_visible(window, probe.taskbar_visible);
        }
        let _ = probe.sender.try_send(result);
        // SAFETY: the probe is serviced only on its owning window's UI thread.
        unsafe { DestroyWindow(window) };
    }
}

fn take(window: Hwnd) -> Option<Probe> {
    let probe = {
        let mut probes = lock_probes();
        probes.remove(&window)
    };
    if probe.is_some() {
        // SAFETY: stopping an absent timer is a no-op; this is the window's UI
        // thread during either timer dispatch or destruction.
        unsafe { KillTimer(window, TIMER_ID) };
    }
    probe
}

fn clear_if_visible(window: Hwnd, taskbar_visible: bool) {
    if taskbar_visible {
        let _ = set_taskbar_progress(window, TaskbarProgress::Clear);
    }
}

fn lock_probes() -> std::sync::MutexGuard<'static, BTreeMap<Hwnd, Probe>> {
    match PROBES.get_or_init(|| Mutex::new(BTreeMap::new())).lock() {
        Ok(probes) => probes,
        // The window procedure contains panics at its boundary. The probe only
        // stores scalar progress state and one sender, so retaining that state
        // for cleanup is safer than leaving a timer or taskbar indicator behind.
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_timeout_is_bounded() {
        let mut state = State::default();
        for _ in 1..READINESS_TIMEOUT_TICKS {
            assert_eq!(state.next(), Next::Wait);
        }
        assert_eq!(state.next(), Next::Fail);
    }

    #[test]
    fn ready_probe_runs_a_fixed_progress_sequence() {
        let mut state = State::default();
        assert!(state.taskbar_button_created());
        for percent in (0..=100).step_by(PROGRESS_STEP.into()) {
            assert_eq!(
                state.next(),
                Next::Present(TaskbarProgress::Determinate {
                    completed: percent,
                    total: PROGRESS_TOTAL,
                })
            );
        }
        assert_eq!(state.next(), Next::ClearAndSucceed);
    }

    #[test]
    fn shell_restart_requires_fresh_readiness_before_progress_resumes() {
        let mut state = State::default();
        assert!(state.taskbar_button_created());
        assert!(matches!(state.next(), Next::Present(_)));
        state.taskbar_restarted();
        assert_eq!(state.next(), Next::Wait);
        assert!(state.taskbar_button_created());
        assert_eq!(
            state.next(),
            Next::Present(TaskbarProgress::Determinate {
                completed: 0,
                total: PROGRESS_TOTAL,
            })
        );
    }
}
