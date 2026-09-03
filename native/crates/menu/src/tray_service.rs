//! One-way host service seam for applying a semantic tray menu.

use std::fmt;

use crate::{ContextMenuModel, TrayRevision};

/// Safe public outcome of applying one complete tray menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayServiceError {
    /// The host has no session tray surface or could not apply the model.
    Unavailable,
}

/// Host-owned bridge that applies one validated complete tray model.
///
/// The core supplies only a revision and semantic menu. A host owns the native
/// icon, popup placement, command numbers, callbacks, and UI-thread routing.
pub trait TrayService: fmt::Debug + Send {
    /// Applies `model` as the complete tray menu at `revision`.
    ///
    /// Failure must leave a previously accepted native menu intact.
    fn replace(
        &self,
        revision: TrayRevision,
        model: ContextMenuModel,
    ) -> Result<(), TrayServiceError>;
}

/// Service used before a host attaches a session-owned tray surface.
#[derive(Debug, Default)]
pub struct UnavailableTrayService;

impl TrayService for UnavailableTrayService {
    fn replace(
        &self,
        _revision: TrayRevision,
        _model: ContextMenuModel,
    ) -> Result<(), TrayServiceError> {
        Err(TrayServiceError::Unavailable)
    }
}
