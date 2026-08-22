//! Bounded semantic native-menu state for one authenticated session.
//!
//! This crate validates the portable menu model and its revision-bound command
//! state. It also defines the narrow one-way host service seam used to apply a
//! validated whole-model replacement. It has no protocol, queue, renderer,
//! operating-system call, native handle, callback, or application identity. A
//! host owns all of those seams.
//! See `docs/MENUS.md` and Decisions 0080 and 0089.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod error;
mod mailbox;
mod model;
mod revision;
mod service;
mod session;
mod shortcut;

pub use error::MenuError;
pub use mailbox::{MENU_RESPONSE_TIMEOUT, MenuMailbox, MenuRequest};
pub use model::{
    MAX_MENU_ITEM_LABEL_BYTES, MAX_MENU_ITEMS, MAX_MENU_LABEL_BYTES, MAX_MENUS, Menu, MenuAction,
    MenuActionId, MenuModel, MenuText,
};
pub use revision::MenuRevision;
pub use service::{MenuService, MenuServiceError, UnavailableMenuService};
pub use session::{MenuActionEvent, MenuSession};
pub use shortcut::MenuShortcut;
