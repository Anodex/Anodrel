//! Test-host constructors with one explicit service surface each.

use super::*;

pub(crate) fn clipboard_host(
    grants: Vec<Capability>,
    clipboard: impl ClipboardService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_clipboard(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        clipboard,
    )
}

pub(crate) fn external_host(
    grants: Vec<Capability>,
    external_links: impl ExternalLinkService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text(None),
        external_links,
    )
}

pub(crate) fn network_host(
    grants: Vec<Capability>,
    network: impl NetworkTextService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        HostServices::unavailable().with_network(network),
    )
}

pub(crate) fn file_dialog_host(
    grants: Vec<Capability>,
    dialogs: impl FileDialogService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_all_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text(None),
        FailingExternalLinks,
        dialogs,
    )
}

pub(crate) fn file_access_host(
    grants: Vec<Capability>,
    selections: impl FileSelectionService + 'static,
    text: impl FileTextService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_all_services_and_file_access(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text(None),
        FailingExternalLinks,
        CancellingFileDialog,
        selections,
        text,
    )
}

pub(crate) fn file_write_host(
    grants: Vec<Capability>,
    selections: impl SaveSelectionService + 'static,
    writer: impl FileTextWriteService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        HostServices::unavailable()
            .with_file_save_selections(selections)
            .with_file_text_write(writer),
    )
}

pub(crate) fn file_binary_write_host(
    grants: Vec<Capability>,
    writer: impl FileBinaryWriteService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        HostServices::unavailable().with_file_binary_write(writer),
    )
}

pub(crate) fn storage_host(
    grants: Vec<Capability>,
    storage: impl StorageService + 'static,
) -> CoreHost {
    CoreHost::with_session_components_and_all_services_and_file_access_and_storage(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        MemoryClipboard::with_text(None),
        FailingExternalLinks,
        CancellingFileDialog,
        CapturingFileDialog,
        FixedFileText(Err(FileTextServiceError::Unavailable)),
        storage,
    )
}

pub(crate) fn credential_host(
    grants: Vec<Capability>,
    credentials: impl CredentialService + 'static,
) -> CoreHost {
    CoreHost::with_credential_service(
        HostPolicy::new("test.application", grants, "test-host").expect("test policy is valid"),
        credentials,
    )
}
