//! Closed typed failures from the UI-session facade.

use std::fmt;

use anodrel_client::ClientError;

/// A safe UI-client failure category.
///
/// No variant retains a document, action ID, request ID, bootstrap record,
/// endpoint name, raw response, or operating-system error. Applications may
/// decide only whether their fixed workflow can continue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiClientError {
    /// The supplied document was not one strict bounded v1 document.
    DocumentInvalid,
    /// The supplied menu was not one strict bounded complete menu model.
    MenuInvalid,
    /// The supplied context menu was not one strict bounded complete model.
    ContextMenuInvalid,
    /// The supplied tray menu was not one strict bounded complete model.
    TrayInvalid,
    /// The supplied notification text failed the fixed portable rules.
    NotificationInvalid,
    /// The supplied host-owned file-dialog filters were invalid.
    FileDialogFiltersInvalid,
    /// The supplied retained-output text exceeded its fixed bound.
    FileTextInvalid,
    /// The supplied secondary-window title failed its fixed portable rules.
    WindowTitleInvalid,
    /// The authenticated underlying conversation could not continue.
    Conversation(ClientError),
    /// A response did not match this typed facade's documented shape.
    ResponseInvalid,
    /// The facade could not generate another unique request identity.
    RequestIdsExhausted,
}

impl fmt::Display for UiClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::DocumentInvalid => "the UI document was invalid",
            Self::MenuInvalid => "the native menu model was invalid",
            Self::ContextMenuInvalid => "the native context-menu model was invalid",
            Self::TrayInvalid => "the native tray model was invalid",
            Self::NotificationInvalid => "the notification text was invalid",
            Self::FileDialogFiltersInvalid => "the file-dialog filters were invalid",
            Self::FileTextInvalid => "the selected-output text was invalid",
            Self::WindowTitleInvalid => "the window title was invalid",
            Self::Conversation(_) => "the authenticated UI conversation could not continue",
            Self::ResponseInvalid => "the host returned an invalid UI response",
            Self::RequestIdsExhausted => "the UI request identity space was exhausted",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UiClientError {}

impl From<ClientError> for UiClientError {
    fn from(error: ClientError) -> Self {
        Self::Conversation(error)
    }
}
