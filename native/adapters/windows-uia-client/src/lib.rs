#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, read-only Windows UI Automation client support for host diagnostics.
//!
//! This adapter is intentionally separate from Anodrel's provider. It lets a
//! host-owned diagnostic read a fixed native window through Windows, without
//! exposing an automation tree, pointer, listener, or result to an application.

mod client;
mod com;
mod raw;

pub use client::{
    UiAutomationClient, UiAutomationElement, UiAutomationError, UiAutomationNode, UiAutomationRect,
};
pub use com::ComApartment;
