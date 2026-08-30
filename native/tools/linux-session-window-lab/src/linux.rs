//! Linux-only fixed child/view lifetime check.

use std::{env, error::Error, path::Path};

use anodrel_core::HostPolicy;
use anodrel_linux_bootstrap::LinuxBootstrapProgram;
use anodrel_linux_development_window::{
    LinuxDevelopmentWindowEvent, LinuxDevelopmentWindowSession,
};
use anodrel_linux_lab_surface::compose;
use anodrel_protocol::Capability;

const SESSION_ID: &str = "linux-session-window-lab";
const HOST_NAME: &str = "anodrel-linux-session-window-lab";

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let program = program_from_arguments()?;
    let policy = HostPolicy::new(HOST_NAME, vec![Capability::DiagnosticsRead], HOST_NAME)?;
    let mut session = LinuxDevelopmentWindowSession::start(policy, SESSION_ID, program)?;
    let run_result = run_view(&mut session);
    let finish_result = session.finish();
    run_result?;
    finish_result?;
    Ok(())
}

fn program_from_arguments() -> Result<LinuxBootstrapProgram, Box<dyn Error>> {
    let mut arguments = env::args_os();
    let _ = arguments.next();
    let Some(path) = arguments.next() else {
        return Err("Linux Session Lab requires one fixed first-party child path".into());
    };
    if arguments.next().is_some() {
        return Err("Linux Session Lab accepts exactly one child path".into());
    }
    LinuxBootstrapProgram::new(Path::new(&path)).map_err(Into::into)
}

fn run_view(session: &mut LinuxDevelopmentWindowSession) -> Result<(), Box<dyn Error>> {
    session.present(&compose(false))?;
    let mut activated = false;
    loop {
        match session.wait_for_event()? {
            LinuxDevelopmentWindowEvent::Activated if !activated => {
                session.present(&compose(true))?;
                activated = true;
            }
            LinuxDevelopmentWindowEvent::Activated => {}
            LinuxDevelopmentWindowEvent::Closed => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::Path};

    #[test]
    fn development_client_argument_stays_one_absolute_path() {
        let path = OsString::from("/opt/anodrel/anodrel-native-linux-session-client");
        assert!(Path::new(&path).is_absolute());
    }
}
