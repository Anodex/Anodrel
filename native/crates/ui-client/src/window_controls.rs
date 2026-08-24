//! Typed, targetless session-window controls.
//!
//! These methods expose existing protocol operations only. Every operation
//! remains bound to the authenticated session's own host window; applications
//! cannot name a native window, inspect its state, or widen its grant.

use std::io::{Read, Write};

use anodrel_client::ProtocolVersion;
use anodrel_json::JsonValue;
use anodrel_window::{WindowFullscreenMode, WindowSize, WindowState, WindowTitleProposal};

use crate::{UiClientError, UiSession};

/// The first protocol version that accepts a session-window title proposal.
const WINDOW_TITLE_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(14);
/// The first protocol version that accepts a closed session-window state.
const WINDOW_STATE_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(16);
/// The first protocol version that accepts a targetless foreground request.
const WINDOW_FOCUS_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(20);
/// The first protocol version that accepts a closed fullscreen mode.
const WINDOW_FULLSCREEN_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(21);
/// The first protocol version that accepts a bounded logical client size.
const WINDOW_SIZE_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(23);

impl<Stream> UiSession<Stream>
where
    Stream: Read + Write,
{
    /// Proposes a title for this session's own host window.
    ///
    /// The host composes any displayed caption with its validated application
    /// name. This method takes no window target and never returns the composed
    /// title or current native title.
    pub fn set_window_title(&mut self, title: &str) -> Result<(), UiClientError> {
        let title =
            WindowTitleProposal::new(title).map_err(|_| UiClientError::WindowTitleInvalid)?;
        let result = self.request(
            WINDOW_TITLE_PROTOCOL,
            "window.title.set",
            JsonValue::Object(
                [(
                    "title".to_owned(),
                    JsonValue::String(title.as_str().to_owned()),
                )]
                .into_iter()
                .collect(),
            ),
        )?;
        expect_status(&result, "applied")
    }

    /// Requests one closed presentation state for this session's own host window.
    ///
    /// A successful result acknowledges the host request only; it does not
    /// report a native window state.
    pub fn set_window_state(&mut self, state: WindowState) -> Result<(), UiClientError> {
        let state = match state {
            WindowState::Minimized => "minimized",
            WindowState::Maximized => "maximized",
            WindowState::Restored => "restored",
        };
        let result = self.request(
            WINDOW_STATE_PROTOCOL,
            "window.state.set",
            JsonValue::Object(
                [("state".to_owned(), JsonValue::String(state.to_owned()))]
                    .into_iter()
                    .collect(),
            ),
        )?;
        expect_status(&result, "applied")
    }

    /// Asks Windows to foreground this session's own host window.
    ///
    /// Acceptance means only that the host asked Windows. It does not report
    /// foreground state, user attention, input, or any other window.
    pub fn request_window_focus(&mut self) -> Result<(), UiClientError> {
        let result = self.request(
            WINDOW_FOCUS_PROTOCOL,
            "window.focus.request",
            JsonValue::Object(Default::default()),
        )?;
        expect_status(&result, "requested")
    }

    /// Requests one reversible presentation mode for this session's own host window.
    ///
    /// Fullscreen remains host-controlled borderless presentation; this method
    /// cannot choose a monitor, display mode, geometry, or native window.
    pub fn set_window_fullscreen(
        &mut self,
        mode: WindowFullscreenMode,
    ) -> Result<(), UiClientError> {
        let mode = match mode {
            WindowFullscreenMode::Fullscreen => "fullscreen",
            WindowFullscreenMode::Windowed => "windowed",
        };
        let result = self.request(
            WINDOW_FULLSCREEN_PROTOCOL,
            "window.fullscreen.set",
            JsonValue::Object(
                [("mode".to_owned(), JsonValue::String(mode.to_owned()))]
                    .into_iter()
                    .collect(),
            ),
        )?;
        expect_status(&result, "applied")
    }

    /// Requests one bounded logical client size for this session's own host window.
    ///
    /// [`WindowSize`] has already checked the inclusive bounds. The host alone
    /// converts those logical dimensions to a native frame and never returns
    /// geometry, DPI, monitor, or presentation state.
    pub fn set_window_size(&mut self, size: WindowSize) -> Result<(), UiClientError> {
        let result = self.request(
            WINDOW_SIZE_PROTOCOL,
            "window.size.set",
            JsonValue::Object(
                [
                    (
                        "width".to_owned(),
                        JsonValue::Number(size.width().to_string()),
                    ),
                    (
                        "height".to_owned(),
                        JsonValue::Number(size.height().to_string()),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
        )?;
        expect_status(&result, "applied")
    }
}

fn expect_status(result: &JsonValue, expected: &str) -> Result<(), UiClientError> {
    if result
        .as_object()
        .and_then(|fields| fields.get("status"))
        .and_then(JsonValue::as_string)
        == Some(expected)
    {
        Ok(())
    } else {
        Err(UiClientError::ResponseInvalid)
    }
}
