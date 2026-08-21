use std::fmt;

use crate::{MenuModel, MenuRevision};

/// The safe public outcome of applying one session menu model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuServiceError {
    /// The host has no session-owned native menu surface, or could not update it.
    Unavailable,
}

/// A host-owned bridge that applies one validated complete menu model.
///
/// The core supplies only an opaque monotonic revision and a portable model.
/// An implementation must own any native command identifiers, window state,
/// and UI-thread routing itself; none are part of this interface. The session
/// core moves to a pipe worker, so implementations must be safe to move there.
pub trait MenuService: fmt::Debug + Send {
    /// Applies `model` as the session's complete native menu at `revision`.
    ///
    /// On failure the implementation must retain its prior complete native
    /// menu. The caller does not advance its portable session state unless this
    /// method succeeds.
    fn replace(&self, revision: MenuRevision, model: MenuModel) -> Result<(), MenuServiceError>;
}

/// A service used until a native host attaches a session-owned menu surface.
#[derive(Debug, Default)]
pub struct UnavailableMenuService;

impl MenuService for UnavailableMenuService {
    fn replace(&self, _revision: MenuRevision, _model: MenuModel) -> Result<(), MenuServiceError> {
        Err(MenuServiceError::Unavailable)
    }
}
