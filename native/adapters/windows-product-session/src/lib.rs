#![forbid(unsafe_code)]

//! Host-owned lifecycle for one verified interactive Windows product session.
//!
//! This adapter joins the already narrow registered-session, locked-launch,
//! pipe-worker, and native-window boundaries. It adds no protocol operation,
//! application-controlled process argument, or public window API.

use std::{
    fmt, io,
    sync::Arc,
    thread::{self, JoinHandle},
};

use anodrel_windows_launch::{LaunchError, TrackedApplication, launch_registered_application};
use anodrel_windows_pipe::{InvitationError, PipeStopSignal, WindowsPipeServer};
use anodrel_windows_registered_session::{
    RegisteredSessionError, RegisteredSessionUi, create_registered_ui_session,
};

const WAIT_FOREVER_MILLISECONDS: u32 = u32::MAX;

/// Starts one verified interactive session for a host-selected application.
///
/// Call this from a host worker before entering the Win32 UI loop. The returned
/// UI group is host-created and must be supplied only to the matching internal
/// authenticated-window entry point.
pub fn start_registered_product_session(
    application_id: &str,
    host_name: impl Into<String>,
    session_id: impl Into<String>,
) -> Result<RunningProductSession, ProductSessionError> {
    let registered = create_registered_ui_session(application_id, host_name, session_id)
        .map_err(ProductSessionError::Session)?;
    let (server, invitation, ui) = registered.into_parts();
    let bootstrap = invitation
        .bootstrap_invitation()
        .map_err(ProductSessionError::Invitation)?;
    let application = Arc::new(
        launch_registered_application(application_id, &bootstrap)
            .map_err(ProductSessionError::Launch)?,
    );
    let pipe_stop = server.stop_signal();

    let pipe_worker = match spawn_pipe_worker(server, Arc::clone(&application), ui.close_signal()) {
        Ok(worker) => worker,
        Err(error) => {
            stop_after_start_failure(&application, &pipe_stop, &ui);
            return Err(ProductSessionError::WorkerStart(error));
        }
    };
    let exit_watcher = match spawn_exit_watcher(
        Arc::clone(&application),
        pipe_stop.clone(),
        ui.close_signal(),
    ) {
        Ok(watcher) => watcher,
        Err(error) => {
            stop_after_start_failure(&application, &pipe_stop, &ui);
            let _ = pipe_worker.join();
            return Err(ProductSessionError::WorkerStart(error));
        }
    };

    Ok(RunningProductSession {
        application,
        pipe_stop,
        ui,
        pipe_worker: Some(pipe_worker),
        exit_watcher: Some(exit_watcher),
    })
}

/// The host-owned state for one running verified Windows product session.
///
/// Keep this value alive while running its matching native window. Call
/// [`Self::finish`] after that window closes to join the two worker threads.
pub struct RunningProductSession {
    application: Arc<TrackedApplication>,
    pipe_stop: PipeStopSignal,
    ui: RegisteredSessionUi,
    pipe_worker: Option<JoinHandle<io::Result<()>>>,
    exit_watcher: Option<JoinHandle<io::Result<u32>>>,
}

impl RunningProductSession {
    /// Returns the grouped UI resources for this exact product session.
    #[must_use]
    pub fn ui(&self) -> &RegisteredSessionUi {
        &self.ui
    }

    /// Requests complete host shutdown for this product session.
    ///
    /// This is idempotent best-effort cleanup. The pipe worker, window, and
    /// tracked child all observe the corresponding host-owned signals.
    pub fn shutdown(&self) {
        self.ui.close_signal().request();
        self.pipe_stop.request_stop();
        let _ = self.application.terminate();
    }

    /// Requests shutdown and joins the pipe worker and child-exit watcher.
    pub fn finish(mut self) -> Result<(), ProductSessionError> {
        self.shutdown();
        let pipe_result = join_pipe_worker(self.pipe_worker.take());
        let watcher_result = join_exit_watcher(self.exit_watcher.take());
        pipe_result?;
        watcher_result
    }
}

impl Drop for RunningProductSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl fmt::Debug for RunningProductSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RunningProductSession(..)")
    }
}

fn spawn_pipe_worker(
    server: WindowsPipeServer,
    application: Arc<TrackedApplication>,
    close_signal: anodrel_core::SessionCloseSignal,
) -> io::Result<JoinHandle<io::Result<()>>> {
    thread::Builder::new()
        .name("anodrel-product-pipe".to_owned())
        .spawn(move || {
            let result = server.serve_one();
            close_signal.request();
            // A product child cannot outlive its authenticated host channel.
            // An already-exited child makes this best-effort cleanup harmless.
            let _ = application.terminate();
            result
        })
}

fn spawn_exit_watcher(
    application: Arc<TrackedApplication>,
    pipe_stop: PipeStopSignal,
    close_signal: anodrel_core::SessionCloseSignal,
) -> io::Result<JoinHandle<io::Result<u32>>> {
    thread::Builder::new()
        .name("anodrel-product-exit".to_owned())
        .spawn(
            move || match application.wait_for_exit(WAIT_FOREVER_MILLISECONDS) {
                Ok(exit_code) => {
                    pipe_stop.request_stop();
                    close_signal.request();
                    Ok(exit_code)
                }
                Err(error) => {
                    // An unobservable child lifetime fails closed: close the window,
                    // release the worker, and request process termination.
                    pipe_stop.request_stop();
                    close_signal.request();
                    let _ = application.terminate();
                    Err(error)
                }
            },
        )
}

fn stop_after_start_failure(
    application: &TrackedApplication,
    pipe_stop: &PipeStopSignal,
    ui: &RegisteredSessionUi,
) {
    ui.close_signal().request();
    pipe_stop.request_stop();
    let _ = application.terminate();
}

fn join_pipe_worker(worker: Option<JoinHandle<io::Result<()>>>) -> Result<(), ProductSessionError> {
    match worker {
        Some(worker) => worker
            .join()
            .map_err(|_| ProductSessionError::WorkerPanicked)?
            .map_err(ProductSessionError::Pipe),
        None => Ok(()),
    }
}

fn join_exit_watcher(
    watcher: Option<JoinHandle<io::Result<u32>>>,
) -> Result<(), ProductSessionError> {
    match watcher {
        Some(watcher) => {
            let _ = watcher
                .join()
                .map_err(|_| ProductSessionError::ExitWatcherPanicked)?
                .map_err(ProductSessionError::ChildExit)?;
            Ok(())
        }
        None => Ok(()),
    }
}

/// Safe failure categories while starting or ending a product session.
#[derive(Debug)]
pub enum ProductSessionError {
    Session(RegisteredSessionError),
    Invitation(InvitationError),
    Launch(LaunchError),
    WorkerStart(io::Error),
    WorkerPanicked,
    Pipe(io::Error),
    ExitWatcherPanicked,
    ChildExit(io::Error),
}

impl fmt::Display for ProductSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Session(_) => "registered product session could not be created",
            Self::Invitation(_) => "registered product bootstrap could not be prepared",
            Self::Launch(_) => "registered product application could not be launched",
            Self::WorkerStart(_) => "registered product worker could not be started",
            Self::WorkerPanicked => "registered product pipe worker stopped unexpectedly",
            Self::Pipe(_) => "registered product pipe worker failed",
            Self::ExitWatcherPanicked => "registered product exit watcher stopped unexpectedly",
            Self::ChildExit(_) => "registered product child exit could not be observed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProductSessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Session(error) => Some(error),
            Self::Invitation(error) => Some(error),
            Self::Launch(error) => Some(error),
            Self::WorkerStart(error) | Self::Pipe(error) | Self::ChildExit(error) => Some(error),
            Self::WorkerPanicked | Self::ExitWatcherPanicked => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_windows_policy::PolicyStoreError;

    use super::{ProductSessionError, start_registered_product_session};

    #[test]
    fn rejects_an_invalid_application_id_before_product_launch() {
        assert!(matches!(
            start_registered_product_session("org.anodrel/escape", "windows-host", "test-session"),
            Err(ProductSessionError::Session(
                anodrel_windows_registered_session::RegisteredSessionError::Policy(
                    PolicyStoreError::InvalidApplicationId
                )
            ))
        ));
    }
}
