//! Direct current-process elevation check for machine-changing installer commands.

use std::fmt;

/// The current process could not prove it holds an elevated Windows token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ElevationError {
    /// Windows did not provide the current process token elevation state.
    TokenUnavailable,
    /// The current process token was not elevated.
    NotElevated,
}

impl fmt::Display for ElevationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TokenUnavailable => "Windows could not verify installer elevation",
            Self::NotElevated => "run this installer command from an elevated shell",
        })
    }
}

impl std::error::Error for ElevationError {}

/// Requires the current process, without relaunching, to hold an elevated token.
pub(super) fn require_elevation() -> Result<(), ElevationError> {
    raw::current_process_elevated()?
        .then_some(())
        .ok_or(ElevationError::NotElevated)
}

mod raw {
    use std::{ffi::c_void, mem};

    use super::ElevationError;

    type Handle = isize;
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION: u32 = 20;

    #[repr(C)]
    struct TokenElevation {
        is_elevated: u32,
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
    }
    #[link(name = "Advapi32")]
    unsafe extern "system" {
        fn OpenProcessToken(process: Handle, access: u32, token: *mut Handle) -> i32;
        fn GetTokenInformation(
            token: Handle,
            information_class: u32,
            information: *mut c_void,
            information_length: u32,
            returned_length: *mut u32,
        ) -> i32;
    }

    pub(super) fn current_process_elevated() -> Result<bool, ElevationError> {
        // SAFETY: GetCurrentProcess returns a valid pseudo-handle for this process.
        let process = unsafe { GetCurrentProcess() };
        let mut token = 0_isize;
        // SAFETY: `token` is one writable output slot and TOKEN_QUERY is sufficient.
        if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
            return Err(ElevationError::TokenUnavailable);
        }
        let _token = TokenHandle(token);
        let mut elevation = TokenElevation { is_elevated: 0 };
        let mut returned_length = 0_u32;
        // SAFETY: TokenElevation is a valid writable buffer for the exact requested class.
        let read = unsafe {
            GetTokenInformation(
                token,
                TOKEN_ELEVATION,
                (&mut elevation as *mut TokenElevation).cast(),
                mem::size_of::<TokenElevation>() as u32,
                &mut returned_length,
            )
        };
        (read != 0 && returned_length == mem::size_of::<TokenElevation>() as u32)
            .then_some(elevation.is_elevated != 0)
            .ok_or(ElevationError::TokenUnavailable)
    }

    struct TokenHandle(Handle);

    impl Drop for TokenHandle {
        fn drop(&mut self) {
            // SAFETY: OpenProcessToken returned this owned access-token handle.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ElevationError;

    #[test]
    fn elevation_failures_are_safe_operator_messages() {
        assert_eq!(
            ElevationError::NotElevated.to_string(),
            "run this installer command from an elevated shell"
        );
        assert_eq!(
            ElevationError::TokenUnavailable.to_string(),
            "Windows could not verify installer elevation"
        );
    }
}
