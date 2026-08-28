//! Accessibility publication and host-private revalidation for registered views.

use super::*;

/// Derives accessibility semantics for whichever view a window carries.
///
/// Only the two views that render a UI document have semantics to publish. A
/// document or Startup Lab window reports none, so assistive technology sees
/// the window itself and nothing inside it.
pub(crate) fn accessibility_snapshot(
    window: Hwnd,
    width: f32,
    height: f32,
) -> io::Result<Option<AccessibilityPublication>> {
    let views = lock_views()?;
    Ok(match views.get(&window) {
        // A UI Lab is host-owned diagnostic state. Its local action tiles have
        // no authenticated-session mailbox, so they are readable but
        // intentionally expose no UI Automation Invoke pattern.
        Some(View::UiLab(lab)) => Some(AccessibilityPublication {
            snapshot: lab.accessibility_snapshot(width, height),
            action_sink: None,
            focus_route: Some(lab.accessibility_focus_route(None)),
            scroll_snapshot: lab.accessibility_scroll_snapshot(width, height),
            scroll_items: lab.accessibility_scroll_items(width, height),
            scroll_route: Some(lab.accessibility_scroll_route(None)),
            focused: lab.accessibility_focus(),
            field_values: lab.accessibility_field_values(),
        }),
        Some(View::UiSession(session)) => Some(AccessibilityPublication {
            snapshot: session.lab().accessibility_snapshot(width, height),
            action_sink: session.accessibility_action_sink(),
            focus_route: Some(session.accessibility_focus_route()),
            scroll_snapshot: session.lab().accessibility_scroll_snapshot(width, height),
            scroll_items: session.lab().accessibility_scroll_items(width, height),
            scroll_route: Some(session.accessibility_scroll_route()),
            focused: session.lab().accessibility_focus(),
            field_values: session.lab().accessibility_field_values(),
        }),
        _ => None,
    })
}

/// Takes and revalidates a private UI Automation focus request on one view.
///
/// It is intentionally not a generic focus API: the only caller is the
/// host's payload-free UIA wake message, and it cannot choose a view or target.
pub(crate) fn service_accessibility_focus(
    window: Hwnd,
    width: f32,
    height: f32,
) -> io::Result<Option<AccessibilityFocusResult>> {
    let mut views = lock_views()?;
    Ok(match views.get_mut(&window) {
        Some(View::UiLab(lab)) => lab.service_accessibility_focus(None, width, height),
        Some(View::UiSession(session)) => session.service_accessibility_focus(width, height),
        _ => None,
    })
}

/// Takes and revalidates a private UI Automation scroll request on one view.
///
/// The caller is the host's payload-free UIA wake message. Its target and
/// command came from the one active host-owned route, not from a window message.
pub(crate) fn service_accessibility_scroll(
    window: Hwnd,
    width: f32,
    height: f32,
) -> io::Result<Option<AccessibilityScrollResult>> {
    let mut views = lock_views()?;
    Ok(match views.get_mut(&window) {
        Some(View::UiLab(lab)) => lab.service_accessibility_scroll(None, width, height),
        Some(View::UiSession(session)) => session.service_accessibility_scroll(width, height),
        _ => None,
    })
}
