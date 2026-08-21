#![forbid(unsafe_code)]

//! A typed, bounded native UI-session facade.
//!
//! This preview crate sits above [`anodrel_client`] for development-native UI
//! applications. It offers exactly document replacement, semantic-action pull,
//! and self-close; it deliberately exposes neither arbitrary protocol requests
//! nor host authority. See `docs/NATIVE_UI_TEMPLATE.md`.

mod error;
mod events;
mod revision;
mod session;

pub use error::UiClientError;
pub use events::{MAX_ACTIONS_PER_BATCH, UiAction, UiActionBatch};
pub use revision::DocumentRevision;
pub use session::UiSession;

#[cfg(test)]
mod tests;
