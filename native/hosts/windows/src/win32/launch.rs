//! Public host launch paths and common message-loop setup.
//!
//! These fixed routes compose host-selected views and resources. They do not
//! form a general native window API and accept no application-provided handle,
//! style, or desktop-control value. Focused child modules keep diagnostics,
//! fixed routes, and raw window lifecycle separate.

mod diagnostics;
mod lifecycle;
mod routes;

#[cfg(debug_assertions)]
pub(super) use diagnostics::crash_selftest;
#[cfg(debug_assertions)]
pub use diagnostics::run_crash_selftest_panic;
pub(super) use diagnostics::startup_log_book;
pub use diagnostics::{print_startup_report, run_crash_report_selftest};
pub(super) use lifecycle::{run_windows, run_windows_after_created, run_windows_after_shown};
pub(super) use routes::ui_lab_window;
pub use routes::{
    run, run_application, run_grouped_ui_session, run_startup_lab, run_ui_lab, run_ui_preview,
    run_uia_property_probe, run_window_group_lab, run_window_lab,
};
