//! Development-only visual verification for session-owned native view groups.
//!
//! This is intentionally a host diagnostic, not a protocol operation. It lets
//! a person exercise the same worker-to-UI creation, per-view routing, and
//! group-close path that Protocol 1.25 will later use without allowing an
//! application to create, target, or inspect any native window.

use std::io;

use anodrel_core::SessionCloseSignal;
use anodrel_ui_session::{UiWindowGroup, UiWindowId};
use anodrel_window::WindowTitleProposal;

use super::{
    View, WindowDefinition, primary_scale, run_windows, session_window_group::SessionWindowGroup,
    ui_session_view::UiSessionView,
};

const PRIMARY_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"group.primary","kind":"stack","axis":"vertical","padding":{"left":48,"top":48,"right":48,"bottom":48},"gap":20,"surfaceTone":"plain","children":[{"id":"group.primary.title","kind":"text","value":"Session-owned Window Group","fontSize":28,"tone":"primary"},{"id":"group.primary.body","kind":"text","value":"This host-owned primary view will create one separately routed secondary window. Close the secondary first to keep this view open. Close this primary first to end the full group.","fontSize":16,"tone":"secondary"}]}}"#;

const SECONDARY_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"group.secondary","kind":"stack","axis":"vertical","padding":{"left":48,"top":48,"right":48,"bottom":48},"gap":20,"surfaceTone":"plain","children":[{"id":"group.secondary.title","kind":"text","value":"Independent Secondary View","fontSize":28,"tone":"primary"},{"id":"group.secondary.body","kind":"text","value":"This view has its own document revision, mailbox, semantic-input queue, native registry entry, and logical identity. It receives no primary-only native service bridge.","fontSize":16,"tone":"secondary"}]}}"#;

/// Opens the fixed host diagnostic described in `docs/WINDOW_LIFECYCLE.md`.
pub(super) fn run() -> io::Result<()> {
    let portable = UiWindowGroup::new();
    let primary_id = UiWindowId::primary();
    portable
        .replace_document(&primary_id, PRIMARY_DOCUMENT)
        .map_err(|_| io::Error::other("the fixed group-lab primary document is invalid"))?;
    let primary_resources = portable
        .resources(&primary_id)
        .ok_or_else(|| io::Error::other("the group-lab primary view is unavailable"))?;
    let group = SessionWindowGroup::new(
        portable.clone(),
        SessionCloseSignal::default(),
        Some("Anodrel Window Group Lab".to_owned()),
    );
    let worker_group = portable.clone();
    let worker = std::thread::Builder::new()
        .name("anodrel-window-group-lab".to_owned())
        .spawn(move || {
            let title = WindowTitleProposal::new("Independent Secondary View")
                .expect("the fixed group-lab title is valid");
            worker_group.open_secondary(title, SECONDARY_DOCUMENT)
        })
        .map_err(|error| io::Error::new(error.kind(), "group-lab worker could not start"))?;
    let scale = primary_scale();
    let result = run_windows(
        vec![WindowDefinition {
            title: "Anodrel Window Group Lab".to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiSession(Box::new(UiSessionView::for_group_member(
                primary_resources,
                group.member(primary_id),
            ))),
        }],
        None,
    );
    // If startup itself failed, no timer may have observed the close signal.
    // Cancelling here keeps the diagnostic worker bounded in every path.
    group.request_shutdown();
    let _ = worker.join();
    result
}

#[cfg(test)]
mod tests {
    use anodrel_ui_document::decode;

    use super::{PRIMARY_DOCUMENT, SECONDARY_DOCUMENT};

    #[test]
    fn fixed_group_lab_documents_match_the_strict_ui_contract() {
        assert!(decode(PRIMARY_DOCUMENT).is_ok());
        assert!(decode(SECONDARY_DOCUMENT).is_ok());
    }
}
