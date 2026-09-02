//! Rendering of the already-defined installed-record format.

use std::path::Path;

use anodrel_json::JsonValue;

use crate::ReleaseManifest;

impl ReleaseManifest {
    /// Renders the version-1.19 or 1.20 machine record for one host-selected package root.
    ///
    /// The caller must validate this record against the extracted package before
    /// writing it. The root comes from installer code, never the embedded
    /// manifest or an application command line.
    #[must_use]
    pub fn render_install_record(&self, package_root: &Path) -> String {
        let capabilities = self
            .capabilities()
            .iter()
            .map(|capability| JsonValue::String(capability.as_str().to_owned()))
            .collect();
        let network_origins = self
            .network_origins()
            .iter()
            .map(|origin| {
                object([
                    ("host", JsonValue::String(origin.hostname().to_owned())),
                    ("port", JsonValue::Number(origin.port().to_string())),
                ])
            })
            .collect();
        let mut fields = vec![
            (
                "recordVersion".to_owned(),
                object([
                    ("major", JsonValue::Number("1".to_owned())),
                    (
                        "minor",
                        JsonValue::Number(
                            if self.update_catalogue_location().is_some() {
                                "20"
                            } else {
                                "19"
                            }
                            .to_owned(),
                        ),
                    ),
                ]),
            ),
            (
                "applicationId".to_owned(),
                JsonValue::String(self.application_id().to_owned()),
            ),
            (
                "packageRoot".to_owned(),
                JsonValue::String(package_root.display().to_string()),
            ),
            (
                "executable".to_owned(),
                object([
                    ("path", JsonValue::String(self.executable_path().to_owned())),
                    (
                        "sha256",
                        JsonValue::String(anodrel_application::sha256::to_lower_hex(
                            self.executable_digest(),
                        )),
                    ),
                ]),
            ),
            (
                "publisher".to_owned(),
                object([(
                    "leafCertificateSha256",
                    JsonValue::String(anodrel_application::sha256::to_lower_hex(
                        self.publisher_fingerprint(),
                    )),
                )]),
            ),
            ("capabilities".to_owned(), JsonValue::Array(capabilities)),
            (
                "networkOrigins".to_owned(),
                JsonValue::Array(network_origins),
            ),
        ];
        if let Some(location) = self.update_catalogue_location() {
            fields.push((
                "updateCatalogue".to_owned(),
                object([
                    (
                        "origin",
                        object([
                            (
                                "host",
                                JsonValue::String(location.origin().hostname().to_owned()),
                            ),
                            (
                                "port",
                                JsonValue::Number(location.origin().port().to_string()),
                            ),
                        ]),
                    ),
                    (
                        "path",
                        JsonValue::String(location.request_path().to_owned()),
                    ),
                ]),
            ));
        }
        JsonValue::Object(fields.into_iter().collect()).to_json()
    }
}

fn object<const N: usize>(fields: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}
