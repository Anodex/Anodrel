#![deny(unsafe_op_in_unsafe_fn)]

//! Bounded, no-data primary-instance coordination for an Anodrel Windows host.
//!
//! This adapter coordinates only a native window for a validated
//! application identity. It is not authentication, command forwarding, or a
//! public application-to-host channel.

mod raw;

use std::{io, thread, time::Duration};

const MAX_APPLICATION_ID_BYTES: usize = 128;
const READINESS_POLL_INTERVAL: Duration = Duration::from_millis(25);
const READINESS_POLL_ATTEMPTS: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceScope {
    Application,
    StartupLab,
}

impl InstanceScope {
    const fn name(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::StartupLab => "startup-lab",
        }
    }
}

pub enum InstanceClaim {
    Primary(PrimaryInstance),
    Existing(ExistingInstance),
}

/// Holds the primary mutex and readiness event until the host window closes.
pub struct PrimaryInstance {
    _mutex: raw::OwnedHandle,
    ready_event: raw::OwnedHandle,
    activation_message: u32,
}

impl PrimaryInstance {
    /// Signals that the primary has successfully created its native window.
    pub fn mark_ready(&self) -> io::Result<()> {
        raw::set_event(&self.ready_event)
    }

    /// Returns the private registered message that the native window handles.
    pub const fn activation_message(&self) -> u32 {
        self.activation_message
    }
}

/// Represents an existing primary without retaining a synchronization handle.
pub struct ExistingInstance {
    ready_event_name: String,
    activation_message: u32,
}

impl ExistingInstance {
    /// Waits for the primary's window readiness, then requests activation.
    ///
    /// The wait is bounded to one second. No arguments, data, or pointers are
    /// sent to the primary process.
    pub fn activate(&self) -> io::Result<()> {
        let ready_event = self.wait_for_ready_event()?;
        match raw::poll_event(&ready_event)? {
            raw::WaitStatus::Signaled => raw::post_activation_message(self.activation_message),
            raw::WaitStatus::Pending => Err(not_ready()),
        }
    }

    fn wait_for_ready_event(&self) -> io::Result<raw::OwnedHandle> {
        let name = wide_null(&self.ready_event_name);
        for attempt in 0..=READINESS_POLL_ATTEMPTS {
            match raw::open_ready_event(&name) {
                Ok(event) => match raw::poll_event(&event)? {
                    raw::WaitStatus::Signaled => return Ok(event),
                    raw::WaitStatus::Pending if attempt == READINESS_POLL_ATTEMPTS => {
                        return Err(not_ready());
                    }
                    raw::WaitStatus::Pending => thread::sleep(READINESS_POLL_INTERVAL),
                },
                Err(error) if raw::is_not_found(&error) && attempt < READINESS_POLL_ATTEMPTS => {
                    thread::sleep(READINESS_POLL_INTERVAL);
                }
                Err(error) if raw::is_not_found(&error) => return Err(not_ready()),
                Err(error) => return Err(error),
            }
        }
        Err(not_ready())
    }
}

/// Claims a native instance from a validated application ID and scope.
pub fn claim(application_id: &str, scope: InstanceScope) -> io::Result<InstanceClaim> {
    if !is_valid_application_id(application_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "application ID is invalid for instance coordination",
        ));
    }

    let mutex_name = object_name("Anodrel.Instance.v1", application_id, scope);
    let ready_event_name = object_name("Anodrel.InstanceReady.v1", application_id, scope);
    let activation_name = object_name("Anodrel.InstanceActivate.v1", application_id, scope);
    let activation_message = raw::register_activation_message(&wide_null(&activation_name))?;
    let (mutex, already_exists) = raw::create_mutex(&wide_null(&mutex_name))?;

    if already_exists {
        drop(mutex);
        return Ok(InstanceClaim::Existing(ExistingInstance {
            ready_event_name,
            activation_message,
        }));
    }

    let ready_event = raw::create_manual_reset_event(&wide_null(&ready_event_name))?;
    Ok(InstanceClaim::Primary(PrimaryInstance {
        _mutex: mutex,
        ready_event,
        activation_message,
    }))
}

fn object_name(namespace: &str, application_id: &str, scope: InstanceScope) -> String {
    format!(r"Local\{namespace}.{}.{}", scope.name(), application_id)
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn is_valid_application_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (3..=MAX_APPLICATION_ID_BYTES).contains(&bytes.len())
        && bytes
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

fn not_ready() -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        "existing Anodrel instance did not become ready",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_application_ids() {
        assert!(claim("not valid", InstanceScope::Application).is_err());
        assert!(claim(".invalid", InstanceScope::Application).is_err());
    }

    #[test]
    fn coordinates_primary_and_existing_instances() {
        let application_id = format!("org.anodrel.instance-test-{}", std::process::id());
        let primary = match claim(&application_id, InstanceScope::Application)
            .expect("primary instance claim succeeds")
        {
            InstanceClaim::Primary(primary) => primary,
            InstanceClaim::Existing(_) => panic!("test instance name is already claimed"),
        };
        let existing = match claim(&application_id, InstanceScope::Application)
            .expect("existing instance claim succeeds")
        {
            InstanceClaim::Existing(existing) => existing,
            InstanceClaim::Primary(_) => panic!("second claim must observe the primary"),
        };

        primary.mark_ready().expect("primary signals readiness");
        existing
            .activate()
            .expect("existing instance receives activation request");
    }

    #[test]
    fn keeps_diagnostic_and_application_scopes_independent() {
        let application_id = format!("org.anodrel.scope-test-{}", std::process::id());
        let application = claim(&application_id, InstanceScope::Application)
            .expect("application scope claim succeeds");
        let startup_lab = claim(&application_id, InstanceScope::StartupLab)
            .expect("startup lab scope claim succeeds");
        assert!(matches!(application, InstanceClaim::Primary(_)));
        assert!(matches!(startup_lab, InstanceClaim::Primary(_)));
    }
}
