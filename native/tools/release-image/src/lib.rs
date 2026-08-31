#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Owned Windows pre-signing release-image assembly.
//!
//! This tool validates an Anodrel release, embeds it into a new PE copy through
//! direct Windows APIs, and verifies the output resource bytes. It does not
//! sign an executable, install an application, write machine policy, or launch
//! a process.

mod build;
mod error;
mod raw;

pub use build::embed_release_image;
pub use error::ReleaseImageError;

#[cfg(test)]
mod tests;
