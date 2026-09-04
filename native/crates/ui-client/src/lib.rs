#![forbid(unsafe_code)]

//! A typed, bounded native UI-session facade.
//!
//! This preview crate sits above [`anodrel_client`] for development-native UI
//! applications. It offers closed document and menu replacements,
//! semantic-action pull, bounded session-owned secondary windows, and
//! self-close; it deliberately exposes neither arbitrary protocol requests nor
//! host authority. See `docs/NATIVE_UI_TEMPLATE.md`,
//! `docs/NATIVE_MENU_TEMPLATE.md`, and `docs/NATIVE_MULTI_WINDOW_TEMPLATE.md`.

mod context_menu_model;
mod context_menu_revision;
mod error;
mod events;
mod fields;
mod menu_model;
mod menu_revision;
mod notification;
mod popup_events;
mod revision;
mod session;
mod tray;
mod tray_model;
mod tray_revision;
mod window_controls;
mod window_id;

pub use anodrel_window::{WindowFullscreenMode, WindowSize, WindowState};
pub use context_menu_revision::ContextMenuRevision;
pub use error::UiClientError;
pub use events::{
    MAX_ACTIONS_PER_BATCH, MAX_WINDOW_ACTIONS_PER_BATCH, UiAction, UiActionBatch, UiEvent,
    UiEventBatch, UiMenuAction, WindowUiAction, WindowUiActionBatch,
};
pub use fields::{UiFieldSnapshot, UiFieldValue};
pub use menu_revision::MenuRevision;
pub use popup_events::{
    UiContextMenuAction, UiContextMenuActionBatch, UiTrayAction, UiTrayActionBatch,
};
pub use revision::DocumentRevision;
pub use session::UiSession;
pub use tray_revision::TrayRevision;
pub use window_id::{SecondaryWindowId, SessionWindowId};

#[cfg(test)]
mod tests;
