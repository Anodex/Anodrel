#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! First-party bounded filesystem authoring for Anodrel release bundles.
//!
//! This crate reads one existing local source tree and creates one fresh bundle
//! file. It neither changes the source tree nor embeds, signs, installs,
//! launches, downloads, or compresses a release.

mod error;
mod output;
mod source;

pub use error::BundleAuthorError;

use std::path::Path;

/// Creates one checked release bundle from an existing source directory.
///
/// Both paths must be absolute. The source must be a normal directory tree of
/// regular UTF-8-named files without links or special entries. The output must
/// be a previously absent file outside that source tree with an existing normal
/// parent directory. The new file is parsed after encoding and synchronized
/// before this function succeeds.
pub fn create_release_bundle(source: &Path, output: &Path) -> Result<(), BundleAuthorError> {
    let source = source::read_source_tree(source, output)?;
    output::write_new_bundle(output, &source)
}

#[cfg(test)]
mod tests;
