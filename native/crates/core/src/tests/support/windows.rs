//! Window-service fixtures shared by focused behavior tests.

use super::*;

/// A window-title service that records the proposals it was handed.
#[derive(Debug, Default)]
pub(crate) struct RecordingWindowTitle {
    pub(crate) applied: std::sync::Mutex<Vec<String>>,
    pub(crate) result: Option<WindowTitleServiceError>,
}

impl WindowTitleService for RecordingWindowTitle {
    fn set_title(&self, proposal: &WindowTitleProposal) -> Result<(), WindowTitleServiceError> {
        if let Some(error) = self.result {
            return Err(error);
        }
        self.applied
            .lock()
            .expect("the test mutex is usable")
            .push(proposal.as_str().to_owned());
        Ok(())
    }
}

pub(crate) fn request_v1_14(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":14}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn host_with_window_title(service: impl WindowTitleService + 'static) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::WindowTitle],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_window_title(service),
    )
}

/// A state service that records only the closed command it was given.
#[derive(Debug, Default)]
pub(crate) struct RecordingWindowState {
    pub(crate) applied: Arc<Mutex<Vec<WindowState>>>,
    pub(crate) result: Option<WindowStateServiceError>,
}

impl WindowStateService for RecordingWindowState {
    fn set_state(&self, state: WindowState) -> Result<(), WindowStateServiceError> {
        if let Some(error) = self.result {
            return Err(error);
        }
        self.applied
            .lock()
            .expect("the test mutex is usable")
            .push(state);
        Ok(())
    }
}

pub(crate) fn request_v1_16(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":16}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_17(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":17}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_18(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":18}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_24(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":24}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_25(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":25}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_26(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":26}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_27(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":27}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_19(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":19}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_20(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":20}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_21(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":21}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_22(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":22}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn request_v1_23(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":23}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

pub(crate) fn host_with_menu(service: impl MenuService + 'static) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", vec![Capability::MenuWrite], "test-host")
            .expect("test policy is valid"),
        HostServices::unavailable().with_menu(service),
    )
}

pub(crate) fn host_with_window_state(service: impl WindowStateService + 'static) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::WindowState],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_window_state(service),
    )
}

/// A pull-only state service that returns one host-owned portable snapshot.
#[derive(Debug)]
pub(crate) struct RecordingWindowStateRead {
    pub(crate) state: WindowState,
    pub(crate) result: Option<WindowStateReadServiceError>,
}

impl Default for RecordingWindowStateRead {
    fn default() -> Self {
        Self {
            state: WindowState::Restored,
            result: None,
        }
    }
}

impl WindowStateReadService for RecordingWindowStateRead {
    fn read_state(&self) -> Result<WindowState, WindowStateReadServiceError> {
        self.result.map_or(Ok(self.state), Err)
    }
}

pub(crate) fn host_with_window_state_read(
    service: impl WindowStateReadService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::WindowStateRead],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_window_state_read(service),
    )
}

#[derive(Debug, Default)]
pub(crate) struct RecordingWindowFocus {
    pub(crate) requested: Arc<Mutex<u8>>,
    pub(crate) result: Option<WindowFocusServiceError>,
}

#[derive(Debug, Default)]
pub(crate) struct RecordingWindowFullscreen {
    pub(crate) applied: Arc<Mutex<Vec<WindowFullscreenMode>>>,
    pub(crate) result: Option<WindowFullscreenServiceError>,
}

impl WindowFullscreenService for RecordingWindowFullscreen {
    fn set_fullscreen(
        &self,
        mode: WindowFullscreenMode,
    ) -> Result<(), WindowFullscreenServiceError> {
        if let Some(error) = self.result {
            return Err(error);
        }
        self.applied
            .lock()
            .expect("the test mutex is usable")
            .push(mode);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct RecordingWindowSize {
    pub(crate) applied: Arc<Mutex<Vec<WindowSize>>>,
    pub(crate) result: Option<WindowSizeServiceError>,
}

impl WindowSizeService for RecordingWindowSize {
    fn set_size(&self, size: WindowSize) -> Result<(), WindowSizeServiceError> {
        if let Some(error) = self.result {
            return Err(error);
        }
        self.applied
            .lock()
            .expect("the test mutex is usable")
            .push(size);
        Ok(())
    }
}

impl WindowFocusService for RecordingWindowFocus {
    fn request_focus(&self) -> Result<(), WindowFocusServiceError> {
        if let Some(error) = self.result {
            return Err(error);
        }
        let requested = &mut *self.requested.lock().expect("the test mutex is usable");
        *requested = requested.saturating_add(1);
        Ok(())
    }
}

pub(crate) fn host_with_window_focus(service: impl WindowFocusService + 'static) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::WindowFocus],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_window_focus(service),
    )
}

pub(crate) fn host_with_window_fullscreen(
    service: impl WindowFullscreenService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::WindowFullscreen],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_window_fullscreen(service),
    )
}

pub(crate) fn host_with_window_size(service: impl WindowSizeService + 'static) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::WindowSize],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_window_size(service),
    )
}
