//! Contract tests for bounded portable window values.

use super::{
    MAX_PROPOSAL_UTF16_UNITS, MAX_WINDOW_CLIENT_HEIGHT, MAX_WINDOW_CLIENT_WIDTH,
    MIN_WINDOW_CLIENT_HEIGHT, MIN_WINDOW_CLIENT_WIDTH, TITLE_SEPARATOR, WindowCommandError,
    WindowSize, WindowSizeInputError, WindowTitleInputError, WindowTitleProposal, compose,
};

fn proposal(value: &str) -> WindowTitleProposal {
    WindowTitleProposal::new(value).expect("the test proposal is valid")
}

#[test]
fn accepts_an_ordinary_document_name() {
    let title = proposal("Quarterly Report.pdf");
    assert_eq!(title.as_str(), "Quarterly Report.pdf");
}

#[test]
fn requires_the_proposal_to_carry_text() {
    assert_eq!(
        WindowTitleProposal::new(""),
        Err(WindowTitleInputError::Empty)
    );
}

#[test]
fn measures_length_in_utf16_units_like_the_native_call() {
    // An emoji is one character, four UTF-8 bytes, and two UTF-16 units.
    // Only the last matches what SetWindowTextW counts.
    let emoji = "\u{1F680}";
    assert_eq!(emoji.chars().count(), 1);
    assert_eq!(emoji.len(), 4);
    assert_eq!(emoji.encode_utf16().count(), 2);

    let exact = emoji.repeat(MAX_PROPOSAL_UTF16_UNITS / 2);
    assert!(WindowTitleProposal::new(exact.clone()).is_ok());
    assert_eq!(
        WindowTitleProposal::new(format!("{exact}{emoji}")),
        Err(WindowTitleInputError::TooLarge)
    );
}

#[test]
fn accepts_the_exact_bound_and_rejects_one_unit_more() {
    assert!(WindowTitleProposal::new("t".repeat(MAX_PROPOSAL_UTF16_UNITS)).is_ok());
    assert_eq!(
        WindowTitleProposal::new("t".repeat(MAX_PROPOSAL_UTF16_UNITS + 1)),
        Err(WindowTitleInputError::TooLarge)
    );
}

#[test]
fn rejects_every_control_character_including_a_line_feed() {
    // A title is a one-line label; a newline could move the visible text away
    // from the validated host suffix.
    for forged in [
        "Report\nWindows Security",
        "Report\r\u{2014} Windows Security",
        "Report\u{1B}[2K",
        "Report\u{0}",
        "Report\u{85}",
    ] {
        assert_eq!(
            WindowTitleProposal::new(forged),
            Err(WindowTitleInputError::ControlCharacter),
            "{forged:?} was accepted"
        );
    }
}

#[test]
fn the_host_suffix_survives_a_proposal_that_impersonates_another_application() {
    // Whatever the proposal claims, the validated application name is last.
    let composed = compose(&proposal("Windows Security"), Some("Anodrel Sample"));
    assert_eq!(composed, "Windows Security \u{2014} Anodrel Sample");
    assert!(composed.ends_with("Anodrel Sample"));
}

#[test]
fn a_proposal_cannot_forge_a_second_suffix_that_outranks_the_real_one() {
    // The real name remains last even if the proposal contains the separator.
    let composed = compose(
        &proposal("Report \u{2014} Some Other App"),
        Some("Anodrel Sample"),
    );
    assert_eq!(
        composed,
        "Report \u{2014} Some Other App \u{2014} Anodrel Sample"
    );
    assert!(composed.ends_with(&format!("{TITLE_SEPARATOR}Anodrel Sample")));
    assert_eq!(composed.matches("Anodrel Sample").count(), 1);
}

#[test]
fn a_session_without_a_validated_name_gets_its_proposal_alone() {
    // An absent identity claim is better than attaching an unfounded name.
    assert_eq!(compose(&proposal("Report"), None), "Report");
    assert_eq!(compose(&proposal("Report"), Some("")), "Report");
    assert_eq!(compose(&proposal("Report"), Some("   ")), "Report");
}

#[test]
fn no_failure_message_repeats_the_text_that_was_refused() {
    // Refused text must not escape through an error or diagnostic surface.
    let secret = "MarkerZQX";
    let errors = [
        WindowTitleProposal::new(String::new()).unwrap_err(),
        WindowTitleProposal::new(format!("{secret}\n")).unwrap_err(),
        WindowTitleProposal::new(secret.repeat(MAX_PROPOSAL_UTF16_UNITS)).unwrap_err(),
    ];
    for error in errors {
        assert!(
            !error.to_string().contains(secret),
            "{error:?} echoed input"
        );
    }
}

#[test]
fn service_failures_describe_the_request_rather_than_the_host() {
    // Native topology and error details stay on the host side of the boundary.
    for error in [WindowCommandError::Unavailable, WindowCommandError::Busy] {
        let message = error.to_string();
        for leaked in ["handle", "hwnd", "0x", "error code", "user32"] {
            assert!(
                !message.to_lowercase().contains(leaked),
                "{error:?} leaks native detail"
            );
        }
    }
}

#[test]
fn size_requires_bounded_logical_client_dimensions() {
    let size = WindowSize::new(800, 600).expect("ordinary client area is valid");
    assert_eq!(size.width(), 800);
    assert_eq!(size.height(), 600);
    assert!(WindowSize::new(MIN_WINDOW_CLIENT_WIDTH, MIN_WINDOW_CLIENT_HEIGHT).is_ok());
    assert!(WindowSize::new(MAX_WINDOW_CLIENT_WIDTH, MAX_WINDOW_CLIENT_HEIGHT).is_ok());
    assert_eq!(
        WindowSize::new(MIN_WINDOW_CLIENT_WIDTH - 1, MIN_WINDOW_CLIENT_HEIGHT),
        Err(WindowSizeInputError::WidthOutOfRange)
    );
    assert_eq!(
        WindowSize::new(MAX_WINDOW_CLIENT_WIDTH + 1, MIN_WINDOW_CLIENT_HEIGHT),
        Err(WindowSizeInputError::WidthOutOfRange)
    );
    assert_eq!(
        WindowSize::new(MIN_WINDOW_CLIENT_WIDTH, MIN_WINDOW_CLIENT_HEIGHT - 1),
        Err(WindowSizeInputError::HeightOutOfRange)
    );
    assert_eq!(
        WindowSize::new(MIN_WINDOW_CLIENT_WIDTH, MAX_WINDOW_CLIENT_HEIGHT + 1),
        Err(WindowSizeInputError::HeightOutOfRange)
    );
}
