//! Fixed local Wayland-Lab shutdown behavior.

use super::{error::LinuxWaylandError, teardown, window::LinuxWaylandLab};

impl LinuxWaylandLab {
    /// Starts the fixed best-effort teardown without waiting for a compositor.
    ///
    /// This does not expose a generic close protocol or a close acknowledgement.
    pub fn close(&mut self) {
        if self.closing {
            return;
        }
        self.closing = true;
        teardown::close(&self.connection, &self.objects, &self.buffers);
    }

    /// Blocks only in the diagnostic's own event loop until the desktop closes it.
    pub fn wait_for_close(&mut self) -> Result<(), LinuxWaylandError> {
        loop {
            if self.wait_for_lab_event()? == super::window::LinuxWaylandLabEvent::Closed {
                return Ok(());
            }
        }
    }

    pub(super) fn require_open(&self) -> Result<(), LinuxWaylandError> {
        (!self.closing)
            .then_some(())
            .ok_or(LinuxWaylandError::DesktopUnavailable)
    }
}

impl Drop for LinuxWaylandLab {
    fn drop(&mut self) {
        self.close();
    }
}
