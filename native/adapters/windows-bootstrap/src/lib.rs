#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows process launch with a one-use, private bootstrap stream.
//!
//! This crate owns only child process and inherited-handle mechanics. The
//! versioned secret record belongs to `anodrel-bootstrap`; callers still own
//! executable trust, lifecycle policy, and application content hosting.

mod command_line;
mod raw;

use std::{fmt, io};

use anodrel_bootstrap::BootstrapInvitation;

pub use command_line::{BootstrapCommand, CommandError};

/// A launched child process. Dropping this value closes the process handle but
/// deliberately does not terminate the child; lifecycle policy belongs above
/// this narrow bootstrap adapter.
pub struct LaunchedProcess {
    handle: raw::OwnedHandle,
}

impl LaunchedProcess {
    /// Waits for the launched child, returning its process exit code.
    pub fn wait_for_exit(&self, timeout_milliseconds: u32) -> io::Result<u32> {
        raw::wait_for_exit(&self.handle, timeout_milliseconds)
    }

    /// Terminates the child with a host-selected exit code.
    ///
    /// The bootstrap adapter does not decide when shutdown occurs; a higher
    /// lifecycle service calls this when its tracked child must stop.
    pub fn terminate(&self, exit_code: u32) -> io::Result<()> {
        raw::terminate(&self.handle, exit_code)
    }
}

impl fmt::Debug for LaunchedProcess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LaunchedProcess")
            .finish_non_exhaustive()
    }
}

/// Launches `command`, delivers exactly one invitation to the child's standard
/// input, and closes the parent write end. The command line never includes the
/// invitation and the adapter does not modify the child environment.
pub fn launch(
    command: &BootstrapCommand,
    invitation: &BootstrapInvitation,
) -> Result<LaunchedProcess, BootstrapLaunchError> {
    let mut frame = invitation
        .encode()
        .map_err(BootstrapLaunchError::Invitation)?;
    let command_line = command.command_line();
    let result = raw::launch_with_bootstrap(command.program(), &command_line, &frame)
        .map(|handle| LaunchedProcess { handle });
    frame.fill(0);
    result.map_err(BootstrapLaunchError::Io)
}

#[derive(Debug)]
pub enum BootstrapLaunchError {
    Command(CommandError),
    Invitation(anodrel_bootstrap::BootstrapError),
    Io(io::Error),
}

impl fmt::Display for BootstrapLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command(error) => write!(formatter, "invalid bootstrap command: {error}"),
            Self::Invitation(error) => write!(formatter, "invalid bootstrap invitation: {error}"),
            Self::Io(error) => write!(formatter, "Windows bootstrap launch failed: {error}"),
        }
    }
}

impl std::error::Error for BootstrapLaunchError {}

impl From<CommandError> for BootstrapLaunchError {
    fn from(error: CommandError) -> Self {
        Self::Command(error)
    }
}

#[cfg(test)]
mod tests {
    use super::BootstrapCommand;

    #[test]
    fn command_line_quotes_space_and_quote_arguments() {
        let command = BootstrapCommand::new("C:\\Program Files\\Anodrel\\client.exe")
            .expect("program is valid")
            .arg("plain")
            .expect("argument is valid")
            .arg("two words")
            .expect("argument is valid")
            .arg("a\\\"b")
            .expect("argument is valid");
        assert_eq!(
            command.command_line(),
            "\"C:\\Program Files\\Anodrel\\client.exe\" plain \"two words\" \"a\\\\\\\"b\""
        );
    }
}
