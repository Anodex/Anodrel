//! Linux development launch boundary over one exact executable and ANLI frame.

mod raw;

use std::{
    ffi::CString,
    fmt,
    os::unix::ffi::OsStrExt,
    path::{Component, Path},
    time::Duration,
};

use anodrel_linux_client::{LinuxBootstrapError, LinuxBootstrapInvitation};

/// One host-selected executable with no argument or shell surface.
pub struct LinuxBootstrapProgram {
    path: CString,
}

impl LinuxBootstrapProgram {
    /// Validates one absolute host-selected development executable path.
    pub fn new(path: &Path) -> Result<Self, LinuxBootstrapLaunchError> {
        if !path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::CurDir | Component::ParentDir | Component::Prefix(_)
                )
            })
        {
            return Err(LinuxBootstrapLaunchError::ProgramInvalid);
        }
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| LinuxBootstrapLaunchError::ProgramInvalid)?;
        Ok(Self { path })
    }
}

impl fmt::Debug for LinuxBootstrapProgram {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LinuxBootstrapProgram(..)")
    }
}

/// One launched development child process without a public process identifier.
pub struct LaunchedProcess {
    process: i32,
}

impl LaunchedProcess {
    /// Waits for the child inside one host-selected timeout.
    pub fn wait_for_exit(&self, timeout: Duration) -> Result<u32, LinuxWaitError> {
        raw::wait_for_exit(self.process, timeout).map_err(LinuxWaitError::from)
    }

    /// Sends the fixed host termination signal.
    pub fn terminate(&self) -> Result<(), LinuxProcessError> {
        raw::terminate(self.process).map_err(|_| LinuxProcessError::Unavailable)
    }
}

impl fmt::Debug for LaunchedProcess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LaunchedProcess(..)")
    }
}

/// Launches one exact executable and consumes its private ANLI invitation.
pub fn launch(
    program: &LinuxBootstrapProgram,
    invitation: LinuxBootstrapInvitation,
) -> Result<LaunchedProcess, LinuxBootstrapLaunchError> {
    let mut frame = invitation
        .encode()
        .map_err(LinuxBootstrapLaunchError::Invitation)?;
    let result = raw::launch(&program.path, &frame).map(|process| LaunchedProcess { process });
    frame.fill(0);
    result.map_err(|_| LinuxBootstrapLaunchError::Unavailable)
}

/// Safe failure category from Linux development-launch setup.
#[derive(Debug)]
pub enum LinuxBootstrapLaunchError {
    /// The host-selected executable path was not a strict absolute path.
    ProgramInvalid,
    /// The host-created ANLI invitation could not be encoded.
    Invitation(LinuxBootstrapError),
    /// Linux could not safely launch or bootstrap the child.
    Unavailable,
}

impl fmt::Display for LinuxBootstrapLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramInvalid => formatter.write_str("Linux child program is invalid"),
            Self::Invitation(_) => formatter.write_str("Linux child invitation is invalid"),
            Self::Unavailable => formatter.write_str("Linux child launch is unavailable"),
        }
    }
}

impl std::error::Error for LinuxBootstrapLaunchError {}

/// Closed failure category from an opaque child wait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxWaitError {
    /// The child did not exit inside the requested host timeout.
    TimedOut,
    /// Linux could not determine the child status.
    Unavailable,
}

impl From<raw::WaitError> for LinuxWaitError {
    fn from(error: raw::WaitError) -> Self {
        match error {
            raw::WaitError::TimedOut => Self::TimedOut,
            raw::WaitError::Unavailable => Self::Unavailable,
        }
    }
}

impl fmt::Display for LinuxWaitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimedOut => formatter.write_str("Linux child did not exit before its timeout"),
            Self::Unavailable => formatter.write_str("Linux child status is unavailable"),
        }
    }
}

impl std::error::Error for LinuxWaitError {}

/// Closed failure category from host-selected child termination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxProcessError {
    /// Linux could not send the fixed termination signal to the child.
    Unavailable,
}

impl fmt::Display for LinuxProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Linux child termination is unavailable")
    }
}

impl std::error::Error for LinuxProcessError {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{LinuxBootstrapLaunchError, LinuxBootstrapProgram};

    #[test]
    fn program_selection_is_absolute_and_path_clean() {
        assert!(LinuxBootstrapProgram::new(Path::new("/opt/anodrel/client")).is_ok());
        assert!(matches!(
            LinuxBootstrapProgram::new(Path::new("client")),
            Err(LinuxBootstrapLaunchError::ProgramInvalid)
        ));
        assert!(matches!(
            LinuxBootstrapProgram::new(Path::new("/opt/anodrel/../client")),
            Err(LinuxBootstrapLaunchError::ProgramInvalid)
        ));
    }

    #[test]
    fn program_debug_does_not_disclose_the_host_selected_path() {
        let program = LinuxBootstrapProgram::new(Path::new("/opt/anodrel/client"))
            .expect("fixture path is valid");
        assert_eq!(format!("{program:?}"), "LinuxBootstrapProgram(..)");
    }
}
