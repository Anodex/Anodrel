//! Typed protocol-1.13 one-way notification operation.

use std::io::{Read, Write};

use anodrel_client::ProtocolVersion;
use anodrel_json::JsonValue;
use anodrel_notifications::{Notification, NotificationBody, NotificationTitle};

use crate::{UiClientError, UiSession};

/// The first protocol version with the bounded one-way notification operation.
const UI_NOTIFICATION_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(13);

impl<Stream> UiSession<Stream>
where
    Stream: Read + Write,
{
    /// Asks the host to hand one bounded notification to its native surface.
    ///
    /// Text is validated locally with the same portable values as the host,
    /// then checked again by the host. Success means only that the host handed
    /// the values to its operating-system adapter; it never reports whether a
    /// person saw, dismissed, or interacted with anything.
    pub fn show_notification(&mut self, title: &str, body: &str) -> Result<(), UiClientError> {
        let notification = Notification::new(
            NotificationTitle::new(title).map_err(|_| UiClientError::NotificationInvalid)?,
            NotificationBody::new(body).map_err(|_| UiClientError::NotificationInvalid)?,
        );
        let result = self.request(
            UI_NOTIFICATION_PROTOCOL,
            "notification.show",
            JsonValue::Object(
                [
                    (
                        "title".to_owned(),
                        JsonValue::String(notification.title().as_str().to_owned()),
                    ),
                    (
                        "body".to_owned(),
                        JsonValue::String(notification.body().as_str().to_owned()),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
        )?;
        if result
            .as_object()
            .and_then(|fields| fields.get("status"))
            .and_then(JsonValue::as_string)
            == Some("shown")
        {
            Ok(())
        } else {
            Err(UiClientError::ResponseInvalid)
        }
    }
}
