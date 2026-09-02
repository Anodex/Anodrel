#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Owned direct Windows signing for checked Anodrel release images.
//!
//! This crate creates and signs one fresh output image with a certificate the
//! release operator selected by exact SHA-256 fingerprint. It does not create a
//! certificate, import a key, alter trust, install, launch, timestamp, or
//! contact a network service.

mod build;
mod error;
mod output;

pub use build::sign_release_image;
pub use error::ReleaseSignError;

#[cfg(test)]
mod tests;
