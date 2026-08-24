#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Windows UI Automation client support for host diagnostics.
//!
//! This adapter is intentionally separate from Anodrel's provider. It lets a
//! host-owned diagnostic inspect a fixed native window through Windows and use
//! its documented fixed focus and authenticated-action checks, without exposing
//! an automation tree, pointer, listener, operation, or result to an
//! application.

mod client;
mod com;
mod raw;

pub use client::{
    UiAutomationClient, UiAutomationElement, UiAutomationError, UiAutomationNode, UiAutomationRect,
    UiAutomationValue,
};
pub use com::ComApartment;
