//! Exact HTTP request routing for the local update acceptance fixture.

use crate::{CATALOGUE_REQUEST_TARGET, INSTALLER_REQUEST_TARGET};

/// One checked local fixture resource selected by an exact request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureResource {
    /// The attached CMS catalogue for the one newer release.
    Catalogue,
    /// The signed installer described by that catalogue.
    Installer,
}

/// Resolves only one bodyless `GET` request to one fixture resource.
///
/// Every method or target other than the two compile-time values returns
/// `None`. The listener maps that result to its fixed not-found response; it
/// never falls back to a filesystem path or caller-controlled file lookup.
#[must_use]
pub fn resolve_request(method: &str, target: &str, has_body: bool) -> Option<FixtureResource> {
    if method != "GET" || has_body {
        return None;
    }
    match target {
        CATALOGUE_REQUEST_TARGET => Some(FixtureResource::Catalogue),
        INSTALLER_REQUEST_TARGET => Some(FixtureResource::Installer),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{FixtureResource, resolve_request};
    use crate::{CATALOGUE_REQUEST_TARGET, INSTALLER_REQUEST_TARGET};

    #[test]
    fn exact_bodyless_gets_select_the_two_fixed_resources() {
        assert_eq!(
            resolve_request("GET", CATALOGUE_REQUEST_TARGET, false),
            Some(FixtureResource::Catalogue)
        );
        assert_eq!(
            resolve_request("GET", INSTALLER_REQUEST_TARGET, false),
            Some(FixtureResource::Installer)
        );
    }

    #[test]
    fn methods_bodies_and_similar_targets_never_select_a_resource() {
        for request in [
            ("POST", CATALOGUE_REQUEST_TARGET, false),
            ("GET", CATALOGUE_REQUEST_TARGET, true),
            ("GET", "/anodrel/local-update/", false),
            ("GET", "/anodrel/local-update/../stable.p7s", false),
            ("GET", "/anodrel/local-update/stable.p7s?extra", false),
            (
                "GET",
                "/anodrel/local-update/releases/0.1.1/other.exe",
                false,
            ),
        ] {
            assert_eq!(resolve_request(request.0, request.1, request.2), None);
        }
    }
}
