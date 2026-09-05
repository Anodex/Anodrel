//! Private UI Automation event listeners for host acceptance diagnostics.

mod focus;
mod live_status;
mod structure;

pub use focus::UiAutomationFocusSubscription;
pub use live_status::UiAutomationLiveStatusSubscription;
pub use structure::UiAutomationStructureSubscription;
