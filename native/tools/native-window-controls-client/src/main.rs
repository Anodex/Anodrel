//! Fixed child for the host-owned window-controls UI Automation diagnostic.
//!
//! The host gives this development-only executable one private invitation on
//! standard input. It publishes a fixed sequence of semantic actions and uses
//! only the existing typed, targetless session-window controls. It receives no
//! native title, state, rectangle, monitor, focus result, or probe result.

#![deny(unsafe_op_in_unsafe_fn)]

use std::{process::ExitCode, thread};

use anodrel_windows_ui_sdk::{
    InteractivePollSchedule, WindowFullscreenMode, WindowSize, WindowState,
    WindowsUiConnectionError, WindowsUiSession,
};

const TITLE_PROPOSAL: &str = "Window controls exercised";
const TITLE_ACTION: &str = "probe.controls.title-action";
const RESIZE_ACTION: &str = "probe.controls.resize-action";
const MAXIMIZE_ACTION: &str = "probe.controls.maximize-action";
const RESTORE_ACTION: &str = "probe.controls.restore-action";
const FOCUS_ACTION: &str = "probe.controls.focus-action";
const FULLSCREEN_ACTION: &str = "probe.controls.fullscreen-action";
const WINDOWED_ACTION: &str = "probe.controls.windowed-action";
const COMPLETE_ACTION: &str = "probe.controls.complete-action";

#[derive(Clone, Copy)]
enum Step {
    Title,
    Resize,
    Maximize,
    Restore,
    Focus,
    Fullscreen,
    Windowed,
    Complete,
}

impl Step {
    const ALL: [Self; 8] = [
        Self::Title,
        Self::Resize,
        Self::Maximize,
        Self::Restore,
        Self::Focus,
        Self::Fullscreen,
        Self::Windowed,
        Self::Complete,
    ];

    const fn action(self) -> &'static str {
        match self {
            Self::Title => TITLE_ACTION,
            Self::Resize => RESIZE_ACTION,
            Self::Maximize => MAXIMIZE_ACTION,
            Self::Restore => RESTORE_ACTION,
            Self::Focus => FOCUS_ACTION,
            Self::Fullscreen => FULLSCREEN_ACTION,
            Self::Windowed => WINDOWED_ACTION,
            Self::Complete => COMPLETE_ACTION,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Title => "Set host-composed title",
            Self::Resize => "Resize client area",
            Self::Maximize => "Maximise window",
            Self::Restore => "Restore window",
            Self::Focus => "Request foreground attention",
            Self::Fullscreen => "Enter fullscreen",
            Self::Windowed => "Return to windowed presentation",
            Self::Complete => "Complete window-controls probe",
        }
    }

    const fn detail(self) -> &'static str {
        match self {
            Self::Title => "The fixed probe begins with a host-composed caption request.",
            Self::Resize => "The caption request was accepted. Request one bounded client size.",
            Self::Maximize => "The host accepted the size request without returning geometry.",
            Self::Restore => "The host accepted maximise. Restore this session window.",
            Self::Focus => "The host accepted restore. Ask Windows for foreground attention.",
            Self::Fullscreen => "The host requested attention without reporting Windows' result.",
            Self::Windowed => "The host accepted borderless fullscreen. Restore its saved frame.",
            Self::Complete => "The host restored its retained windowed presentation.",
        }
    }
}

#[derive(Clone, Copy)]
enum Stage {
    Completed,
    BootstrapUnreadable,
    EndpointUnavailable,
    AuthenticationRejected,
    DocumentRejected,
    EventReadFailed,
    ActionNotObserved,
    TitleRejected,
    StateRejected,
    FocusRejected,
    FullscreenRejected,
    SizeRejected,
    CloseRejected,
}

impl Stage {
    const fn code(self) -> u8 {
        match self {
            Self::Completed => 0,
            Self::BootstrapUnreadable => 113,
            Self::EndpointUnavailable => 114,
            Self::AuthenticationRejected => 115,
            Self::DocumentRejected => 116,
            Self::EventReadFailed => 117,
            Self::ActionNotObserved => 118,
            Self::TitleRejected => 119,
            Self::StateRejected => 120,
            Self::FocusRejected => 121,
            Self::FullscreenRejected => 122,
            Self::SizeRejected => 123,
            Self::CloseRejected => 124,
        }
    }
}

fn main() -> ExitCode {
    ExitCode::from(run().code())
}

fn run() -> Stage {
    let mut session = match WindowsUiSession::connect_from_stdin() {
        Ok(session) => session,
        Err(WindowsUiConnectionError::BootstrapUnavailable) => return Stage::BootstrapUnreadable,
        Err(WindowsUiConnectionError::InvitedEndpointUnavailable) => {
            return Stage::EndpointUnavailable;
        }
        Err(WindowsUiConnectionError::AuthenticationUnavailable) => {
            return Stage::AuthenticationRejected;
        }
    };

    for (index, step) in Step::ALL.into_iter().enumerate() {
        let revision = match session.replace_document_v1(&document(step)) {
            Ok(revision) if revision.value() == index as u64 + 1 => revision.value(),
            _ => return Stage::DocumentRejected,
        };
        if wait_for_action(&mut session, step.action(), revision).is_err() {
            return Stage::ActionNotObserved;
        }
        let result = match step {
            Step::Title => session.set_window_title(TITLE_PROPOSAL),
            Step::Resize => session.set_window_size(WindowSize::new(960, 640).expect("fixed size")),
            Step::Maximize => session.set_window_state(WindowState::Maximized),
            Step::Restore => session.set_window_state(WindowState::Restored),
            Step::Focus => session.request_window_focus(),
            Step::Fullscreen => session.set_window_fullscreen(WindowFullscreenMode::Fullscreen),
            Step::Windowed => session.set_window_fullscreen(WindowFullscreenMode::Windowed),
            Step::Complete => break,
        };
        if result.is_err() {
            return failure_for(step);
        }
    }
    if session.close().is_err() {
        Stage::CloseRejected
    } else {
        Stage::Completed
    }
}

const fn failure_for(step: Step) -> Stage {
    match step {
        Step::Title => Stage::TitleRejected,
        Step::Resize => Stage::SizeRejected,
        Step::Maximize | Step::Restore => Stage::StateRejected,
        Step::Focus => Stage::FocusRejected,
        Step::Fullscreen | Step::Windowed => Stage::FullscreenRejected,
        Step::Complete => Stage::EventReadFailed,
    }
}

fn wait_for_action(
    session: &mut WindowsUiSession,
    expected_action: &str,
    expected_revision: u64,
) -> Result<(), ()> {
    for interval in InteractivePollSchedule::new() {
        let batch = session.read_actions().map_err(|_| ())?;
        if batch.dropped() != 0 || batch.discarded() != 0 {
            return Err(());
        }
        match batch.actions() {
            [] => thread::sleep(interval),
            [action]
                if action.action() == expected_action
                    && action.revision().value() == expected_revision =>
            {
                return Ok(());
            }
            _ => return Err(()),
        }
    }
    Err(())
}

fn document(step: Step) -> String {
    format!(
        concat!(
            r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"probe.controls.root","kind":"stack","axis":"vertical","padding":{{"left":56,"top":56,"right":56,"bottom":56}},"gap":16,"surfaceTone":"plain","children":["#,
            r#"{{"id":"probe.controls.eyebrow","kind":"text","value":"WINDOW CONTROLS UI AUTOMATION PROBE","fontSize":14,"tone":"accent"}},"#,
            r#"{{"id":"probe.controls.title","kind":"text","value":"Host-owned control verification","fontSize":28,"tone":"primary"}},"#,
            r#"{{"id":"probe.controls.detail","kind":"text","value":"{}","fontSize":16,"tone":"secondary"}},"#,
            r#"{{"id":"{}","kind":"action","label":"{}","fontSize":16,"enabled":true,"tone":"accent"}}]}}}}"#
        ),
        step.detail(),
        step.action(),
        step.label(),
    )
}

#[cfg(test)]
mod tests {
    use anodrel_windows_ui_sdk::WindowSize;

    use super::{COMPLETE_ACTION, Stage, Step, document};

    #[test]
    fn fixed_documents_decode_and_match_their_action() {
        for step in Step::ALL {
            let document = document(step);
            assert!(document.contains(&format!(r#""id":"{}""#, step.action())));
            anodrel_ui_document::decode(&document).expect("fixed probe document must decode");
        }
        assert_eq!(COMPLETE_ACTION, "probe.controls.complete-action");
    }

    #[test]
    fn fixed_requested_size_is_inside_the_public_bound() {
        assert_eq!(WindowSize::new(960, 640).unwrap().width(), 960);
    }

    #[test]
    fn completion_is_the_only_success_exit() {
        assert_eq!(Stage::Completed.code(), 0);
        assert_ne!(Stage::CloseRejected.code(), 0);
    }
}
