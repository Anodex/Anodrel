//! Closed development-diagnostic request kinds and their exact grants.

use anodrel_protocol::Capability;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SampleDialogRequest {
    None,
    OpenFile,
    OpenFolder,
    OpenFolderWithReference,
    OpenFileWithReference,
    SaveFile,
    SaveFileWithReference,
    SaveFileBinaryWithReference,
    Storage,
    Scroll,
    Diagnostics,
    Credentials,
    Notification,
    WindowTitle,
    WindowState,
    WindowFocus,
    WindowFullscreen,
    WindowSize,
    WindowSizeWhileFullscreen,
    FieldRead,
    Menu,
    LiveStatus,
}

/// Returns the exact grant set for one compiled development diagnostic.
pub(super) fn sample_capabilities(dialog_request: SampleDialogRequest) -> Vec<Capability> {
    let mut capabilities = vec![
        Capability::DiagnosticsRead,
        Capability::UiDocumentWrite,
        Capability::UiEventsRead,
        Capability::SessionClose,
        Capability::ClipboardRead,
        Capability::ClipboardWrite,
        Capability::ExternalOpen,
        Capability::DialogOpenFile,
        Capability::DialogSaveFile,
        Capability::FileReadText,
        Capability::FileWriteText,
        Capability::StorageStateRead,
        Capability::StorageStateReplace,
        Capability::StorageStateClear,
        Capability::CredentialRead,
        Capability::CredentialWrite,
        Capability::CredentialDelete,
        Capability::NotificationShow,
        Capability::MenuWrite,
        Capability::WindowTitle,
        Capability::UiFieldsRead,
        Capability::WindowState,
    ];
    // New authority is never silently added to an older broad diagnostic. The
    // focus and fullscreen routes are explicit so each manual check proves the
    // exact grant rather than silently widening a broad diagnostic.
    if matches!(dialog_request, SampleDialogRequest::WindowFocus) {
        capabilities.push(Capability::WindowFocus);
    }
    if matches!(
        dialog_request,
        SampleDialogRequest::OpenFolder | SampleDialogRequest::OpenFolderWithReference
    ) {
        capabilities.push(Capability::DialogOpenFolder);
    }
    if matches!(dialog_request, SampleDialogRequest::OpenFolderWithReference) {
        capabilities.push(Capability::FolderReadEntries);
    }
    if matches!(
        dialog_request,
        SampleDialogRequest::WindowFullscreen | SampleDialogRequest::WindowSizeWhileFullscreen
    ) {
        capabilities.push(Capability::WindowFullscreen);
    }
    if matches!(
        dialog_request,
        SampleDialogRequest::WindowSize | SampleDialogRequest::WindowSizeWhileFullscreen
    ) {
        capabilities.push(Capability::WindowSize);
    }
    if matches!(
        dialog_request,
        SampleDialogRequest::SaveFileBinaryWithReference
    ) {
        capabilities.push(Capability::FileWriteBinary);
    }
    capabilities
}

#[cfg(test)]
mod tests {
    use anodrel_protocol::Capability;

    use super::{SampleDialogRequest, sample_capabilities};

    #[test]
    fn binary_output_grant_is_limited_to_the_explicit_binary_diagnostic() {
        let ordinary = sample_capabilities(SampleDialogRequest::None);
        let binary = sample_capabilities(SampleDialogRequest::SaveFileBinaryWithReference);

        assert!(!ordinary.contains(&Capability::FileWriteBinary));
        assert!(binary.contains(&Capability::FileWriteBinary));
        assert_eq!(binary.len(), ordinary.len() + 1);
        assert!(!binary.contains(&Capability::WindowFocus));
        assert!(!binary.contains(&Capability::WindowFullscreen));
        assert!(!binary.contains(&Capability::WindowSize));
    }

    #[test]
    fn folder_selection_grant_is_limited_to_the_folder_diagnostic() {
        let ordinary = sample_capabilities(SampleDialogRequest::None);
        let folder = sample_capabilities(SampleDialogRequest::OpenFolder);

        assert!(!ordinary.contains(&Capability::DialogOpenFolder));
        assert!(folder.contains(&Capability::DialogOpenFolder));
        assert_eq!(folder.len(), ordinary.len() + 1);
    }

    #[test]
    fn folder_entry_grant_is_limited_to_the_retained_folder_diagnostic() {
        let ordinary = sample_capabilities(SampleDialogRequest::OpenFolder);
        let retained = sample_capabilities(SampleDialogRequest::OpenFolderWithReference);

        assert!(!ordinary.contains(&Capability::FolderReadEntries));
        assert!(retained.contains(&Capability::FolderReadEntries));
        assert_eq!(retained.len(), ordinary.len() + 1);
    }

    #[test]
    fn window_size_grant_is_limited_to_the_explicit_size_diagnostic() {
        let ordinary = sample_capabilities(SampleDialogRequest::None);
        let size = sample_capabilities(SampleDialogRequest::WindowSize);

        assert!(!ordinary.contains(&Capability::WindowSize));
        assert!(size.contains(&Capability::WindowSize));
        assert_eq!(size.len(), ordinary.len() + 1);
        assert!(!size.contains(&Capability::WindowFullscreen));
    }

    #[test]
    fn fullscreen_size_refusal_diagnostic_has_only_its_two_window_grants() {
        let ordinary = sample_capabilities(SampleDialogRequest::None);
        let combined = sample_capabilities(SampleDialogRequest::WindowSizeWhileFullscreen);

        assert!(combined.contains(&Capability::WindowFullscreen));
        assert!(combined.contains(&Capability::WindowSize));
        assert_eq!(combined.len(), ordinary.len() + 2);
    }

    #[test]
    fn live_status_uses_the_existing_document_and_action_grants() {
        assert_eq!(
            sample_capabilities(SampleDialogRequest::LiveStatus),
            sample_capabilities(SampleDialogRequest::None)
        );
    }
}
