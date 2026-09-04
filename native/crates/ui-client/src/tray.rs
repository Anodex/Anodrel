//! Typed protocol-1.33 tray operations for one authenticated UI session.

use std::io::{Read, Write};

use anodrel_client::ProtocolVersion;
use anodrel_json::JsonValue;

use crate::{
    TrayRevision, UiClientError, UiSession, UiTrayActionBatch, tray_model::decode_tray_model,
};

/// The first protocol version with host-owned semantic tray menus.
const UI_TRAY_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(33);

impl<Stream> UiSession<Stream>
where
    Stream: Read + Write,
{
    /// Replaces this session's complete host-owned native tray menu model.
    ///
    /// The model is locally checked before transport. It has no icon, tooltip,
    /// coordinate, callback, command number, focus result, or handle; the host
    /// owns the notification-area entry and all local interaction.
    pub fn replace_tray_v1(&mut self, menu: &str) -> Result<TrayRevision, UiClientError> {
        let result = self.request(UI_TRAY_PROTOCOL, "tray.replace", decode_tray_model(menu)?)?;
        result
            .as_object()
            .and_then(|fields| fields.get("revision"))
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)
            .and_then(TrayRevision::parse)
    }

    /// Drains one batch containing only host-owned tray actions.
    ///
    /// This dedicated protocol-1.33 reader prevents established document,
    /// menu, and context-menu APIs from silently accepting a tray event.
    pub fn read_tray_actions(&mut self) -> Result<UiTrayActionBatch, UiClientError> {
        self.request(
            UI_TRAY_PROTOCOL,
            "ui.events.read",
            JsonValue::Object(Default::default()),
        )
        .and_then(|result| UiTrayActionBatch::parse(&result))
    }
}
