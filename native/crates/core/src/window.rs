//! Narrow session-window command handlers.
//!
//! Every operation in this module reaches only the requester's native window
//! through an injected host service. No operation can name or inspect a window.

use super::*;

impl CoreHost {
    /// Proposes the title of this session's own window.
    ///
    /// The request names no window. The host resolves it from the authenticated
    /// session, and the service composes the displayed caption with a validated
    /// application-name suffix the proposal cannot suppress or forge. Success
    /// reports acceptance only: returning the composed caption would hand the
    /// application a way to probe the host's framing, and it already knows both
    /// halves. See `docs/WINDOW_TITLE.md` and Decision 0066.
    pub(super) fn handle_window_title_set(&self, request: RequestEnvelope) -> JsonValue {
        let Some(title) = window_title_set_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.title.set requires one title string.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowTitle) {
            return self.capability_denied(request.request_id, "window.title.set");
        }

        // A rejected proposal must not become a way to have the host repeat
        // text back: the failure names the rule, never the value.
        let Ok(proposal) = WindowTitleProposal::new(title) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::WindowTitleInvalid,
                "window.title.set title is invalid.",
                None,
            );
        };

        match self.window_title.set_title(&proposal) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("applied".to_owned()))]),
            ),
            Err(WindowTitleServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no window is available to title.",
                None,
            ),
            Err(WindowTitleServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window title change is already pending.",
                None,
            ),
        }
    }

    /// Applies one closed presentation state to this session's own window.
    ///
    /// There is no target and no state readback. The worker gives the portable
    /// enum to a service that must route it to the owning UI thread; success is
    /// acceptance only, never an observation about the native window.
    pub(super) fn handle_window_state_set(&self, request: RequestEnvelope) -> JsonValue {
        let Some(state) = window_state_set_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.state.set requires one closed state string.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowState) {
            return self.capability_denied(request.request_id, "window.state.set");
        }

        match self.window_state.set_state(state) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("applied".to_owned()))]),
            ),
            Err(WindowStateServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no session window is available for the requested state.",
                None,
            ),
            Err(WindowStateServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window state change is already pending.",
                None,
            ),
        }
    }

    /// Applies one reversible fullscreen mode to this session's own window.
    ///
    /// The payload is one closed mode rather than a monitor, rectangle,
    /// display-mode, style, or native command. The host resolves the window
    /// from the authenticated session and retains all restoration facts; a
    /// success response is action acceptance, not fullscreen-state readback.
    pub(super) fn handle_window_fullscreen_set(&self, request: RequestEnvelope) -> JsonValue {
        let Some(mode) = window_fullscreen_set_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.fullscreen.set requires one closed mode string.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowFullscreen) {
            return self.capability_denied(request.request_id, "window.fullscreen.set");
        }

        match self.window_fullscreen.set_fullscreen(mode) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("applied".to_owned()))]),
            ),
            Err(WindowFullscreenServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no session window is available for the requested fullscreen mode.",
                None,
            ),
            Err(WindowFullscreenServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window fullscreen change is already pending.",
                None,
            ),
        }
    }

    /// Requests one bounded logical client size for this session's own window.
    ///
    /// The request carries neither a native rectangle nor a target. The host
    /// service resolves the one session window, converts the logical client
    /// dimensions at its current DPI, and returns acceptance only. No response
    /// becomes geometry, monitor, DPI, or presentation-state readback.
    pub(super) fn handle_window_size_set(&self, request: RequestEnvelope) -> JsonValue {
        let Some(size) = window_size_set_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.size.set requires one bounded logical width and height.",
                None,
            );
        };
        if !self.policy.has(Capability::WindowSize) {
            return self.capability_denied(request.request_id, "window.size.set");
        }

        match self.window_size.set_size(size) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("applied".to_owned()))]),
            ),
            Err(WindowSizeServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no session window is available for the requested client size.",
                None,
            ),
            Err(WindowSizeServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window size change is already pending.",
                None,
            ),
        }
    }

    /// Asks Windows to foreground this session's own window.
    ///
    /// The payload is exactly `{}` because a target, retry option, native
    /// handle, or input action would turn this narrow request into a general
    /// window or desktop-control surface. The result is only that the host
    /// asked Windows; it deliberately contains no focus or activation state.
    pub(super) fn handle_window_focus_request(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "window.focus.request accepts no payload fields.",
                None,
            );
        }
        if !self.policy.has(Capability::WindowFocus) {
            return self.capability_denied(request.request_id, "window.focus.request");
        }

        match self.window_focus.request_focus() {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("requested".to_owned()))]),
            ),
            Err(WindowFocusServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowUnavailable,
                "no session window is available for a foreground request.",
                None,
            ),
            Err(WindowFocusServiceError::Busy) => self.failure(
                request.request_id,
                ProtocolErrorCode::WindowBusy,
                "a window focus request is already pending.",
                None,
            ),
        }
    }
}

/// Reads the exact one-field payload `window.title.set` accepts.
///
/// Extra fields stay invalid, so a future target, position, size, or native
/// style cannot be smuggled into this session-owned window command.
fn window_title_set_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("title"))
        .flatten()
        .and_then(JsonValue::as_string)
}

/// Reads the exact one-field payload `window.state.set` accepts.
///
/// Extra fields are a mismatch rather than a future target, geometry, focus, or
/// native-command escape hatch. The value itself is a closed portable enum, so
/// the core never receives an operating-system state code.
fn window_state_set_payload(value: &JsonValue) -> Option<WindowState> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    match fields.get("state")?.as_string()? {
        "minimized" => Some(WindowState::Minimized),
        "maximized" => Some(WindowState::Maximized),
        "restored" => Some(WindowState::Restored),
        _ => None,
    }
}

/// Reads the exact one-field payload `window.fullscreen.set` accepts.
///
/// Extra fields are a mismatch rather than a future monitor, display-mode,
/// geometry, style, z-order, or input escape hatch. The value itself remains a
/// closed portable mode, so the core never receives a native presentation code.
fn window_fullscreen_set_payload(value: &JsonValue) -> Option<WindowFullscreenMode> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    match fields.get("mode")?.as_string()? {
        "fullscreen" => Some(WindowFullscreenMode::Fullscreen),
        "windowed" => Some(WindowFullscreenMode::Windowed),
        _ => None,
    }
}

/// Reads the exact two-field payload `window.size.set` accepts.
///
/// A position, target, monitor, native rectangle, DPI, constraint, animation,
/// or readback selector must not be smuggled into the small client-size command.
/// Both values are strict non-negative JSON integers before the portable bounded
/// logical client-area type accepts them.
fn window_size_set_payload(value: &JsonValue) -> Option<WindowSize> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let width = u32::from(fields.get("width")?.as_u16()?);
    let height = u32::from(fields.get("height")?.as_u16()?);
    WindowSize::new(width, height).ok()
}
