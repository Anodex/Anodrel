#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Direct Windows HTTP Server API publisher for one local update fixture.
//!
//! It receives no publication, URL, endpoint, certificate, or command
//! argument. The fixed local publication crate validates the only two files;
//! this tool then serves one at a time through the Windows HTTP Server API.

mod error;
mod preflight;
mod raw;

use std::{fs::File, path::Path};

use anodrel_local_update_fixture::{
    FixturePublication, FixtureResource, publication_root, resolve_request,
};

pub use error::LocalUpdateFixtureServerError;

/// A bound no-argument Windows HTTPS fixture server.
pub struct LocalUpdateFixtureServer {
    publication: FixturePublication,
    _candidate: anodrel_windows_installer::VerifiedInstallerImage,
    queue: raw::RequestQueue,
}

impl LocalUpdateFixtureServer {
    /// Binds the fixed Windows HTTPS URL after validating the two fixed files.
    ///
    /// The caller must have run the separate elevated preparation that creates
    /// the temporary Windows TLS binding and URL reservation. This function
    /// creates no certificate, trust entry, registry value, or directory.
    pub fn bind() -> Result<Self, LocalUpdateFixtureServerError> {
        let local_data = anodrel_windows_paths::local_application_data_root()
            .map_err(|_| LocalUpdateFixtureServerError::Publication)?;
        let publication = FixturePublication::open(&publication_root(&local_data))
            .map_err(|_| LocalUpdateFixtureServerError::Publication)?;
        let candidate = preflight::verify(&publication)
            .map_err(|_| LocalUpdateFixtureServerError::Publication)?;
        let queue =
            raw::RequestQueue::bind().map_err(|_| LocalUpdateFixtureServerError::Listener)?;
        Ok(Self {
            publication,
            _candidate: candidate,
            queue,
        })
    }

    /// Serves the fixed request set until the process is stopped.
    ///
    /// Each request is handled synchronously and has no mutable control route.
    /// A malformed, body-bearing, non-GET, or unmatched request receives the
    /// same fixed not-found reply as an absent artifact.
    pub fn serve(self) -> Result<(), LocalUpdateFixtureServerError> {
        loop {
            let request = self
                .queue
                .receive()
                .map_err(|_| LocalUpdateFixtureServerError::Listener)?;
            match resolve_request(request.method(), request.target(), request.has_body()) {
                Some(FixtureResource::Catalogue) => {
                    serve_file(&self.queue, request, self.publication.catalogue())?;
                }
                Some(FixtureResource::Installer) => {
                    serve_file(&self.queue, request, self.publication.installer())?;
                }
                None => self
                    .queue
                    .send_not_found(request)
                    .map_err(|_| LocalUpdateFixtureServerError::Listener)?,
            }
        }
    }
}

fn serve_file(
    queue: &raw::RequestQueue,
    request: raw::ReceivedRequest,
    artifact: (&Path, u64),
) -> Result<(), LocalUpdateFixtureServerError> {
    let file = File::open(artifact.0).map_err(|_| LocalUpdateFixtureServerError::Publication)?;
    queue
        .send_file(request, &file, artifact.1)
        .map_err(|_| LocalUpdateFixtureServerError::Listener)
}
