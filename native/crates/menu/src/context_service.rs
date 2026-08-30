use std::fmt;

use crate::{ContextMenuModel, ContextMenuRevision};

/// The safe public outcome of applying one session context-menu model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextMenuServiceError {
    /// The host has no session-owned native context-menu surface, or could not update it.
    Unavailable,
}

/// A host-owned bridge that applies one validated complete context-menu model.
///
/// The core supplies only an opaque monotonic revision and portable model. A
/// host owns native popup construction, local placement, command identifiers,
/// and UI-thread routing; none are part of this interface.
pub trait ContextMenuService: fmt::Debug + Send {
    /// Applies `model` as the session's complete context menu at `revision`.
    ///
    /// On failure the implementation must retain its prior complete native
    /// model. The caller does not advance portable state unless this succeeds.
    fn replace(
        &self,
        revision: ContextMenuRevision,
        model: ContextMenuModel,
    ) -> Result<(), ContextMenuServiceError>;
}

/// A service used until a native host attaches a session-owned context-menu surface.
#[derive(Debug, Default)]
pub struct UnavailableContextMenuService;

impl ContextMenuService for UnavailableContextMenuService {
    fn replace(
        &self,
        _revision: ContextMenuRevision,
        _model: ContextMenuModel,
    ) -> Result<(), ContextMenuServiceError> {
        Err(ContextMenuServiceError::Unavailable)
    }
}
