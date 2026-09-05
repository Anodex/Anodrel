//! Safe failure categories for registered-session setup.

use std::{fmt, io};

use anodrel_windows_paths::WindowsPathsError;
use anodrel_windows_policy::PolicyStoreError;

/// A safe failure category while creating a registered application session.
#[derive(Debug)]
pub enum RegisteredSessionError {
    Policy(PolicyStoreError),
    InvalidHostName,
    Directories(WindowsPathsError),
    Pipe(io::Error),
}

impl fmt::Display for RegisteredSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(_) => {
                formatter.write_str("registered application policy could not be loaded")
            }
            Self::InvalidHostName => {
                formatter.write_str("registered application session host name is invalid")
            }
            Self::Directories(_) => formatter
                .write_str("registered application service directories could not be derived"),
            Self::Pipe(_) => {
                formatter.write_str("registered application session could not be created")
            }
        }
    }
}

impl std::error::Error for RegisteredSessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            Self::Directories(error) => Some(error),
            Self::Pipe(error) => Some(error),
            Self::InvalidHostName => None,
        }
    }
}
