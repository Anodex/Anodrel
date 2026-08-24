//! Notification-specific test fixtures.

use super::*;

pub(in crate::tests) fn host_with_notifications(
    service: impl NotificationService + 'static,
) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::NotificationShow],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_notifications(service),
    )
}

/// A notification service that records what it was asked to show.
#[derive(Debug, Default)]
pub(in crate::tests) struct RecordingNotifications {
    pub(in crate::tests) shown: std::sync::Mutex<Vec<(String, String)>>,
    pub(in crate::tests) result: Option<NotificationServiceError>,
}

impl RecordingNotifications {
    pub(in crate::tests) fn failing(error: NotificationServiceError) -> Self {
        Self {
            shown: std::sync::Mutex::new(Vec::new()),
            result: Some(error),
        }
    }
}

impl NotificationService for RecordingNotifications {
    fn show(&self, notification: &Notification) -> Result<(), NotificationServiceError> {
        if let Some(error) = self.result {
            return Err(error);
        }
        self.shown
            .lock()
            .expect("the fixture lock is usable")
            .push((
                notification.title().as_str().to_owned(),
                notification.body().as_str().to_owned(),
            ));
        Ok(())
    }
}

pub(in crate::tests) fn notification_payload(title: &str, body: &str) -> String {
    object([
        ("body", JsonValue::String(body.to_owned())),
        ("title", JsonValue::String(title.to_owned())),
    ])
    .to_json()
}
