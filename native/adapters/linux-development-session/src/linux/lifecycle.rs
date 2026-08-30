//! One owner for a Linux development child and its authenticated worker.

use std::{
    fmt, io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anodrel_core::{HostPolicy, SessionCloseSignal};
use anodrel_linux_bootstrap::{
    LaunchedProcess, LinuxBootstrapLaunchError, LinuxBootstrapProgram, LinuxWaitError, launch,
};
use anodrel_linux_pipe::{InvitationError, LinuxPipeServer, LinuxPipeStopSignal};

const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const TERMINATION_GRACE: Duration = Duration::from_millis(250);

/// Starts one development-only Linux child under one host-owned lifetime.
///
/// The program must already have passed the launch adapter's host-selected
/// path validation. This coordinator does not validate executable identity or
/// create a Linux native window.
pub fn start_development_session(
    policy: HostPolicy,
    session_id: impl Into<String>,
    program: LinuxBootstrapProgram,
) -> Result<RunningLinuxDevelopmentSession, LinuxDevelopmentSessionError> {
    let (server, invitation) = LinuxPipeServer::create(policy, session_id)
        .map_err(LinuxDevelopmentSessionError::PipeCreate)?;
    let bootstrap = invitation
        .bootstrap_invitation()
        .map_err(LinuxDevelopmentSessionError::Invitation)?;
    let application =
        Arc::new(launch(&program, bootstrap).map_err(LinuxDevelopmentSessionError::Launch)?);
    let pipe_stop = server.stop_signal();
    let close_signal = SessionCloseSignal::default();
    let shutdown_requested = Arc::new(AtomicBool::new(false));

    let pipe_worker = match spawn_pipe_worker(
        server,
        Arc::clone(&application),
        pipe_stop.clone(),
        close_signal.clone(),
        Arc::clone(&shutdown_requested),
    ) {
        Ok(worker) => worker,
        Err(error) => {
            stop_after_start_failure(&application, &pipe_stop, &close_signal, &shutdown_requested);
            return Err(LinuxDevelopmentSessionError::WorkerStart(error));
        }
    };
    let exit_watcher = match spawn_exit_watcher(
        Arc::clone(&application),
        pipe_stop.clone(),
        close_signal.clone(),
        Arc::clone(&shutdown_requested),
    ) {
        Ok(watcher) => watcher,
        Err(error) => {
            stop_after_start_failure(&application, &pipe_stop, &close_signal, &shutdown_requested);
            let _ = pipe_worker.join();
            return Err(LinuxDevelopmentSessionError::WorkerStart(error));
        }
    };

    Ok(RunningLinuxDevelopmentSession {
        application,
        pipe_stop,
        close_signal,
        shutdown_requested,
        pipe_worker: Some(pipe_worker),
        exit_watcher: Some(exit_watcher),
    })
}

/// Host-owned state for one active Linux development child session.
///
/// Keep this value alive while a future native window owns the matching
/// session. Both explicit `finish` and `Drop` stop the transport, end the
/// tracked child, and join the worker pair.
pub struct RunningLinuxDevelopmentSession {
    application: Arc<LaunchedProcess>,
    pipe_stop: LinuxPipeStopSignal,
    close_signal: SessionCloseSignal,
    shutdown_requested: Arc<AtomicBool>,
    pipe_worker: Option<JoinHandle<io::Result<()>>>,
    exit_watcher: Option<JoinHandle<Result<u32, LinuxWaitError>>>,
}

impl RunningLinuxDevelopmentSession {
    /// Returns the coalescing host-local request made when either child or pipe ends.
    #[must_use]
    pub fn close_signal(&self) -> SessionCloseSignal {
        self.close_signal.clone()
    }

    /// Requests complete host shutdown without exposing native child control.
    pub fn shutdown(&self) {
        request_shutdown(
            &self.application,
            &self.pipe_stop,
            &self.close_signal,
            &self.shutdown_requested,
        );
    }

    /// Requests shutdown and joins every worker under a closed failure category.
    pub fn finish(mut self) -> Result<(), LinuxDevelopmentSessionError> {
        let pipe_result = join_pipe_worker(self.stop_and_take_pipe_worker());
        let watcher_result = join_exit_watcher(self.exit_watcher.take());
        pipe_result?;
        watcher_result
    }

    fn stop_and_take_pipe_worker(&mut self) -> Option<JoinHandle<io::Result<()>>> {
        self.shutdown();
        self.pipe_worker.take()
    }
}

impl Drop for RunningLinuxDevelopmentSession {
    fn drop(&mut self) {
        let _ = join_pipe_worker(self.stop_and_take_pipe_worker());
        let _ = join_exit_watcher(self.exit_watcher.take());
    }
}

impl fmt::Debug for RunningLinuxDevelopmentSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RunningLinuxDevelopmentSession(..)")
    }
}

fn spawn_pipe_worker(
    server: LinuxPipeServer,
    application: Arc<LaunchedProcess>,
    pipe_stop: LinuxPipeStopSignal,
    close_signal: SessionCloseSignal,
    shutdown_requested: Arc<AtomicBool>,
) -> io::Result<JoinHandle<io::Result<()>>> {
    thread::Builder::new()
        .name("anodrel-linux-session-pipe".to_owned())
        .spawn(move || {
            let result = server.serve_one();
            request_shutdown(&application, &pipe_stop, &close_signal, &shutdown_requested);
            result
        })
}

fn spawn_exit_watcher(
    application: Arc<LaunchedProcess>,
    pipe_stop: LinuxPipeStopSignal,
    close_signal: SessionCloseSignal,
    shutdown_requested: Arc<AtomicBool>,
) -> io::Result<JoinHandle<Result<u32, LinuxWaitError>>> {
    thread::Builder::new()
        .name("anodrel-linux-session-exit".to_owned())
        .spawn(move || {
            let result = observe_child(&application, &shutdown_requested);
            pipe_stop.request_stop();
            close_signal.request();
            if result.is_err() {
                let _ = application.force_terminate();
            }
            result
        })
}

fn observe_child(
    application: &LaunchedProcess,
    shutdown_requested: &AtomicBool,
) -> Result<u32, LinuxWaitError> {
    let mut shutdown_started = None;
    let mut force_sent = false;
    loop {
        match application.wait_for_exit(EXIT_POLL_INTERVAL) {
            Ok(exit_code) => return Ok(exit_code),
            Err(LinuxWaitError::TimedOut) => {
                if shutdown_requested.load(Ordering::Acquire) {
                    let started = shutdown_started.get_or_insert_with(Instant::now);
                    if !force_sent && started.elapsed() >= TERMINATION_GRACE {
                        let _ = application.force_terminate();
                        force_sent = true;
                    }
                }
            }
            Err(error) => return Err(error),
        }
    }
}

fn request_shutdown(
    application: &LaunchedProcess,
    pipe_stop: &LinuxPipeStopSignal,
    close_signal: &SessionCloseSignal,
    shutdown_requested: &AtomicBool,
) {
    shutdown_requested.store(true, Ordering::Release);
    close_signal.request();
    pipe_stop.request_stop();
    let _ = application.terminate();
}

fn stop_after_start_failure(
    application: &LaunchedProcess,
    pipe_stop: &LinuxPipeStopSignal,
    close_signal: &SessionCloseSignal,
    shutdown_requested: &AtomicBool,
) {
    request_shutdown(application, pipe_stop, close_signal, shutdown_requested);
    let _ = application.force_terminate();
    // No exit watcher exists on a partial-start failure, so this path reaps the
    // exact child itself instead of leaving a zombie for its parent host.
    let _ = application.wait_for_exit(TERMINATION_GRACE);
}

fn join_pipe_worker(
    worker: Option<JoinHandle<io::Result<()>>>,
) -> Result<(), LinuxDevelopmentSessionError> {
    match worker {
        Some(worker) => worker
            .join()
            .map_err(|_| LinuxDevelopmentSessionError::WorkerPanicked)?
            .map_err(LinuxDevelopmentSessionError::Pipe),
        None => Ok(()),
    }
}

fn join_exit_watcher(
    watcher: Option<JoinHandle<Result<u32, LinuxWaitError>>>,
) -> Result<(), LinuxDevelopmentSessionError> {
    match watcher {
        Some(watcher) => {
            let _ = watcher
                .join()
                .map_err(|_| LinuxDevelopmentSessionError::ExitWatcherPanicked)?
                .map_err(LinuxDevelopmentSessionError::ChildExit)?;
            Ok(())
        }
        None => Ok(()),
    }
}

/// Closed failure category for one Linux development-session lifecycle.
#[derive(Debug)]
pub enum LinuxDevelopmentSessionError {
    PipeCreate(io::Error),
    Invitation(InvitationError),
    Launch(LinuxBootstrapLaunchError),
    WorkerStart(io::Error),
    WorkerPanicked,
    Pipe(io::Error),
    ExitWatcherPanicked,
    ChildExit(LinuxWaitError),
}

impl fmt::Display for LinuxDevelopmentSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::PipeCreate(_) => "Linux development session transport could not be created",
            Self::Invitation(_) => "Linux development session bootstrap could not be prepared",
            Self::Launch(_) => "Linux development session child could not be launched",
            Self::WorkerStart(_) => "Linux development session worker could not be started",
            Self::WorkerPanicked => "Linux development session pipe worker stopped unexpectedly",
            Self::Pipe(_) => "Linux development session pipe worker failed",
            Self::ExitWatcherPanicked => {
                "Linux development session exit watcher stopped unexpectedly"
            }
            Self::ChildExit(_) => "Linux development session child exit could not be observed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LinuxDevelopmentSessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PipeCreate(error) | Self::WorkerStart(error) | Self::Pipe(error) => Some(error),
            Self::Invitation(error) => Some(error),
            Self::Launch(error) => Some(error),
            Self::ChildExit(error) => Some(error),
            Self::WorkerPanicked | Self::ExitWatcherPanicked => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io, thread};

    use super::{LinuxDevelopmentSessionError, join_exit_watcher, join_pipe_worker};

    #[test]
    fn completed_workers_do_not_need_a_second_join() {
        assert!(join_pipe_worker(None).is_ok());
        assert!(join_exit_watcher(None).is_ok());
    }

    #[test]
    fn worker_failures_stay_closed() {
        let worker = thread::spawn(|| Err(io::Error::other("private Linux failure")));
        assert!(matches!(
            join_pipe_worker(Some(worker)),
            Err(LinuxDevelopmentSessionError::Pipe(_))
        ));
    }

    #[test]
    fn failure_display_does_not_disclose_native_details() {
        let error = LinuxDevelopmentSessionError::Pipe(io::Error::other("native detail"));
        assert_eq!(
            error.to_string(),
            "Linux development session pipe worker failed"
        );
    }
}
