//! Fixed, host-owned Linux development child and Wayland-Lab lifetime.

use std::{fmt, time::Duration};

use anodrel_canvas::Canvas;
use anodrel_core::{HostPolicy, SessionCloseSignal};
use anodrel_linux_bootstrap::LinuxBootstrapProgram;
use anodrel_linux_development_session::{
    LinuxDevelopmentSessionError, RunningLinuxDevelopmentSession, start_development_session,
};
use anodrel_linux_wayland::{LinuxWaylandError, LinuxWaylandLab, LinuxWaylandLabEvent};

const SESSION_CLOSE_WAIT: Duration = Duration::from_millis(50);

/// One closed outcome from the fixed Linux development window session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxDevelopmentWindowEvent {
    /// The fixed local pointer activation completed.
    Activated,
    /// The private session or compositor ended the fixed view.
    Closed,
}

/// One fixed Wayland Lab retained with one host-owned development session.
pub struct LinuxDevelopmentWindowSession {
    lab: LinuxWaylandLab,
    session: RunningLinuxDevelopmentSession,
    close_signal: SessionCloseSignal,
}

impl LinuxDevelopmentWindowSession {
    /// Starts one private development session and opens its fixed Lab view.
    pub fn start(
        policy: HostPolicy,
        session_id: impl Into<String>,
        program: LinuxBootstrapProgram,
    ) -> Result<Self, LinuxDevelopmentWindowError> {
        let session = start_development_session(policy, session_id, program)
            .map_err(LinuxDevelopmentWindowError::Session)?;
        let close_signal = session.close_signal();
        let lab = LinuxWaylandLab::open().map_err(LinuxDevelopmentWindowError::Desktop)?;
        Ok(Self {
            lab,
            session,
            close_signal,
        })
    }

    /// Presents one complete fixed-size first-party diagnostic canvas.
    pub fn present(&mut self, canvas: &Canvas) -> Result<(), LinuxDevelopmentWindowError> {
        self.lab
            .present(canvas)
            .map_err(LinuxDevelopmentWindowError::Desktop)
    }

    /// Waits for a local Lab outcome or the private child/session close signal.
    ///
    /// Idle waiting is bounded to one fixed kernel wait. Neither the close
    /// source nor raw compositor input is exposed to the child or protocol.
    pub fn wait_for_event(
        &mut self,
    ) -> Result<LinuxDevelopmentWindowEvent, LinuxDevelopmentWindowError> {
        loop {
            match poll_lab_event(&self.close_signal, |timeout| {
                self.lab.wait_for_lab_event_timeout(timeout)
            })
            .map_err(LinuxDevelopmentWindowError::Desktop)?
            {
                PollOutcome::TimedOut => {}
                PollOutcome::SessionClosed | PollOutcome::Event(LinuxWaylandLabEvent::Closed) => {
                    return Ok(LinuxDevelopmentWindowEvent::Closed);
                }
                PollOutcome::Event(LinuxWaylandLabEvent::Activated) => {
                    return Ok(LinuxDevelopmentWindowEvent::Activated);
                }
            }
        }
    }

    /// Drops the fixed view before stopping and joining its private session.
    pub fn finish(self) -> Result<(), LinuxDevelopmentWindowError> {
        let Self {
            lab,
            session,
            close_signal,
        } = self;
        drop(lab);
        drop(close_signal);
        session
            .finish()
            .map_err(LinuxDevelopmentWindowError::Session)
    }
}

impl fmt::Debug for LinuxDevelopmentWindowSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LinuxDevelopmentWindowSession(..)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PollOutcome {
    TimedOut,
    SessionClosed,
    Event(LinuxWaylandLabEvent),
}

fn poll_lab_event(
    close_signal: &SessionCloseSignal,
    wait: impl FnOnce(Duration) -> Result<Option<LinuxWaylandLabEvent>, LinuxWaylandError>,
) -> Result<PollOutcome, LinuxWaylandError> {
    if close_signal.take() {
        return Ok(PollOutcome::SessionClosed);
    }
    let event = wait(SESSION_CLOSE_WAIT)?;
    if close_signal.take() {
        return Ok(PollOutcome::SessionClosed);
    }
    Ok(match event {
        Some(event) => PollOutcome::Event(event),
        None => PollOutcome::TimedOut,
    })
}

/// Closed setup, desktop, or shutdown failure from one development Lab.
#[derive(Debug)]
pub enum LinuxDevelopmentWindowError {
    Session(LinuxDevelopmentSessionError),
    Desktop(LinuxWaylandError),
}

impl fmt::Display for LinuxDevelopmentWindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(_) => formatter.write_str("Linux development session is unavailable"),
            Self::Desktop(_) => formatter.write_str("Linux development window is unavailable"),
        }
    }
}

impl std::error::Error for LinuxDevelopmentWindowError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Session(error) => Some(error),
            Self::Desktop(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LinuxWaylandLabEvent, PollOutcome, SESSION_CLOSE_WAIT, poll_lab_event};
    use anodrel_core::SessionCloseSignal;

    #[test]
    fn pending_session_close_skips_the_compositor_wait() {
        let close_signal = SessionCloseSignal::default();
        close_signal.request();
        let outcome = poll_lab_event(&close_signal, |_| panic!("must not wait after close"))
            .expect("close has no desktop failure");
        assert_eq!(outcome, PollOutcome::SessionClosed);
    }

    #[test]
    fn close_arriving_during_the_fixed_wait_wins_over_an_idle_timeout() {
        let close_signal = SessionCloseSignal::default();
        let request = close_signal.clone();
        let outcome = poll_lab_event(&close_signal, |timeout| {
            assert_eq!(timeout, SESSION_CLOSE_WAIT);
            request.request();
            Ok(None)
        })
        .expect("host close has no desktop failure");
        assert_eq!(outcome, PollOutcome::SessionClosed);
    }

    #[test]
    fn compositor_events_remain_closed_semantic_outcomes() {
        let close_signal = SessionCloseSignal::default();
        let outcome = poll_lab_event(&close_signal, |_| Ok(Some(LinuxWaylandLabEvent::Activated)))
            .expect("fixed compositor event is accepted");
        assert_eq!(outcome, PollOutcome::Event(LinuxWaylandLabEvent::Activated));
    }
}
