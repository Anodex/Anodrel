//! Direct User32 context-popup construction and private command mapping.
//!
//! This module owns the short-lived native popup used for a semantic session
//! context menu. Applications cannot choose its location, native handle,
//! command identifiers, or invocation mechanism.

use std::{collections::BTreeMap, ptr, sync::Arc};

use anodrel_menu::{ContextMenuRequest, ContextMenuRevision, MenuActionId};
use anodrel_ui_session::ContextMenuInputCandidate;

use super::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, Hmenu, Hwnd, Lparam, TrackPopupMenu, Uint,
    to_wide_null,
};

/// The host-private range for the at-most-16 popup commands in one session.
///
/// Semantic action IDs never select these values. They exist only while one
/// host-created popup is displayed on its owning UI thread.
const FIRST_CONTEXT_COMMAND_ID: u16 = 0x7100;
const MF_GRAYED: Uint = 0x0001;
const TPM_RIGHTBUTTON: Uint = 0x0002;
const TPM_RETURNCMD: Uint = 0x0100;

/// One complete host-retained semantic context menu.
///
/// It contains no native User32 object. A popup is built only while Windows is
/// showing it and is destroyed before this call returns.
#[derive(Clone)]
pub(super) struct ContextMenu {
    inner: Arc<ContextMenuInner>,
}

struct ContextMenuInner {
    revision: ContextMenuRevision,
    items: Vec<ContextMenuItem>,
    enabled_actions: BTreeMap<u16, MenuActionId>,
}

struct ContextMenuItem {
    command: u16,
    label: String,
    enabled: bool,
}

/// The host-only outcome after Windows was offered one local popup.
pub(super) enum ContextMenuDisplay {
    /// The user opened and dismissed the popup without choosing an action.
    Dismissed,
    /// The selected private command mapped to one current semantic action.
    Selected(ContextMenuInputCandidate),
}

impl ContextMenu {
    /// Builds and probes one complete native popup before retaining the model.
    ///
    /// The probe occurs before the view registry replaces its current model.
    /// A User32 allocation or label-conversion failure therefore leaves the
    /// prior model untouched and answers the worker with `menu.unavailable`.
    pub(super) fn build(request: &ContextMenuRequest) -> Option<Self> {
        let mut next_command = FIRST_CONTEXT_COMMAND_ID;
        let mut items = Vec::with_capacity(request.model().items().len());
        let mut enabled_actions = BTreeMap::new();
        for action in request.model().items() {
            let command = next_command;
            next_command = next_command.checked_add(1)?;
            if action.enabled() {
                enabled_actions.insert(command, action.id().clone());
            }
            items.push(ContextMenuItem {
                command,
                label: action.label().as_str().to_owned(),
                enabled: action.enabled(),
            });
        }
        let menu = Self {
            inner: Arc::new(ContextMenuInner {
                revision: request.revision(),
                items,
                enabled_actions,
            }),
        };
        // Prove the current User32 environment can construct every item
        // before the portable replacement commits. The temporary handle is
        // destroyed by `PopupMenu` immediately after this check.
        let _ = PopupMenu::build(&menu)?;
        Some(menu)
    }

    /// Shows a popup only for a pointer-originated `WM_CONTEXTMENU` message.
    ///
    /// Keyboard-originated messages carry `(-1, -1)` and deliberately retain
    /// normal Windows handling. They cannot become an application-controlled
    /// local trigger through this surface.
    pub(super) fn show_for_pointer(
        &self,
        window: Hwnd,
        lparam: Lparam,
    ) -> Option<ContextMenuDisplay> {
        let (x, y) = pointer_screen_position(lparam)?;
        let popup = PopupMenu::build(self)?;
        // SAFETY: the menu is a complete host-owned popup, the coordinates
        // came only from this current User32 message, and the null exclusion
        // rectangle means no application geometry reaches the call.
        let command = unsafe {
            TrackPopupMenu(
                popup.handle,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                x,
                y,
                0,
                window,
                ptr::null(),
            )
        };
        let candidate = u16::try_from(command)
            .ok()
            .and_then(|command| self.candidate_for_command(command));
        Some(match candidate {
            Some(candidate) => ContextMenuDisplay::Selected(candidate),
            None => ContextMenuDisplay::Dismissed,
        })
    }

    /// Maps only one current enabled private command to a semantic candidate.
    fn candidate_for_command(&self, command: u16) -> Option<ContextMenuInputCandidate> {
        let action = self.inner.enabled_actions.get(&command)?.clone();
        Some(ContextMenuInputCandidate::new(self.inner.revision, action))
    }
}

/// One transient native popup menu, destroyed when its scope ends.
struct PopupMenu {
    handle: Hmenu,
}

impl PopupMenu {
    /// Builds one complete User32 popup from a retained semantic model.
    fn build(menu: &ContextMenu) -> Option<Self> {
        // SAFETY: CreatePopupMenu requires no input and returns either a new
        // empty User32 popup or a null handle.
        let handle = unsafe { CreatePopupMenu() };
        if handle == 0 {
            return None;
        }
        let popup = Self { handle };
        for item in &menu.inner.items {
            let label = to_wide_null(&escape_mnemonics(&item.label));
            let flags = if item.enabled { 0 } else { MF_GRAYED };
            // SAFETY: `popup` owns this User32 menu, `command` is private to
            // this model, and `label` is live null-terminated UTF-16.
            if unsafe {
                AppendMenuW(
                    popup.handle,
                    flags,
                    usize::from(item.command),
                    label.as_ptr(),
                )
            } == 0
            {
                return None;
            }
        }
        Some(popup)
    }
}

impl Drop for PopupMenu {
    fn drop(&mut self) {
        // SAFETY: each nonzero handle comes from CreatePopupMenu and belongs
        // only to this short-lived value. It has never been attached to a
        // window or another parent menu.
        unsafe { DestroyMenu(self.handle) };
    }
}

/// Decodes User32's screen coordinates from a context-menu message.
///
/// `(-1, -1)` is Windows' keyboard-origin marker and intentionally has no
/// popup route. Coordinates are signed 16-bit values in the documented packed
/// message format, so negative multi-monitor positions remain correct.
pub(super) fn pointer_screen_position(lparam: Lparam) -> Option<(i32, i32)> {
    let packed = lparam as usize;
    let x = i32::from((packed as u16) as i16);
    let y = i32::from(((packed >> 16) as u16) as i16);
    ((x, y) != (-1, -1)).then_some((x, y))
}

/// Escapes User32 mnemonic markers in application display text.
fn escape_mnemonics(value: &str) -> String {
    value.replace('&', "&&")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use anodrel_menu::{ContextMenuRevision, MenuActionId};

    use super::{ContextMenu, ContextMenuInner, FIRST_CONTEXT_COMMAND_ID, pointer_screen_position};

    fn menu() -> ContextMenu {
        ContextMenu {
            inner: std::sync::Arc::new(ContextMenuInner {
                revision: ContextMenuRevision::INITIAL
                    .next()
                    .expect("the first revision exists"),
                items: Vec::new(),
                enabled_actions: BTreeMap::from([(
                    FIRST_CONTEXT_COMMAND_ID,
                    MenuActionId::new("document.rename").expect("fixed ID is valid"),
                )]),
            }),
        }
    }

    fn packed_position(x: i16, y: i16) -> isize {
        let x = usize::from(x as u16);
        let y = usize::from(y as u16) << 16;
        (x | y) as isize
    }

    #[test]
    fn keeps_signed_pointer_coordinates_and_ignores_keyboard_origin() {
        assert_eq!(
            pointer_screen_position(packed_position(-18, 42)),
            Some((-18, 42))
        );
        assert_eq!(pointer_screen_position(packed_position(-1, -1)), None);
    }

    #[test]
    fn maps_only_a_current_enabled_private_command() {
        let candidate = menu()
            .candidate_for_command(FIRST_CONTEXT_COMMAND_ID)
            .expect("the private command is mapped");
        let (revision, action) = candidate.into_parts();
        assert_eq!(revision.value(), 1);
        assert_eq!(action.as_str(), "document.rename");
        assert!(menu().candidate_for_command(0x7FFF).is_none());
    }
}
