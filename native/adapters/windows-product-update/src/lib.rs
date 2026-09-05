#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! One native-user-initiated Windows product-update controller.
//!
//! A verified product window owns this controller after the person chooses its
//! fixed system-menu action. Application code has no route to construct or
//! invoke it. Discovery and installation run on owned workers; the caller polls
//! on its UI thread to present the existing native consent and safe terminal
//! outcome. See `docs/PRODUCT_UPDATES.md` and Decision 0199.

mod controller;

pub use anodrel_windows_update_consent::{UpdateConsent, request_update_consent};
pub use controller::{
    ProductUpdateController, ProductUpdateOutcome, ProductUpdatePoll, ProductUpdateStartError,
};
