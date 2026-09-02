//! Fixed no-argument composition for one signed initial installation.

use anodrel_windows_install_consent::{InitialInstallConsent, request_initial_install_consent};
use anodrel_windows_install_handoff::begin_elevated_initial_install;
use anodrel_windows_installer::prepare_current_initial_install;

/// Runs the only interactive first-install route without accepting input.
pub(super) fn run() -> Result<String, String> {
    let prepared = prepare_current_initial_install().map_err(display_error)?;
    let approval = match request_initial_install_consent(prepared).map_err(display_error)? {
        InitialInstallConsent::Approved(approval) => approval,
        InitialInstallConsent::Declined => {
            return Ok("Anodrel installation was not started.".to_owned());
        }
    };
    let completed = begin_elevated_initial_install(approval)
        .map_err(display_error)?
        .wait()
        .map_err(display_error)?;
    completed.verify_installation().map_err(display_error)?;
    Ok("Current signed Anodrel release installed.".to_owned())
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn an_unsigned_test_image_stops_before_consent_or_uac() {
        assert_eq!(
            run(),
            Err("the signed installer release is invalid".to_owned())
        );
    }
}
