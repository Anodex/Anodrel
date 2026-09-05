#![deny(unsafe_op_in_unsafe_fn)]

mod command;
mod development_ui_session;
mod native_context_menu_template;
mod native_file_binary_write_template;
mod native_file_write_template;
mod native_form_template;
mod native_live_status_template;
mod native_menu_template;
mod native_multi_window_template;
mod native_network_probe;
mod native_notification_template;
mod native_probe;
mod native_scroll_window_template;
mod native_template;
mod native_tray_template;
mod native_ui_probe;
mod native_window_controls_template;
mod product;
mod sample;
mod session_ui;
mod startup;
mod uia_invoke_probe;
mod uia_live_status_event_probe;
mod uia_structure_event_probe;
mod win32;

use std::{env, error::Error, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    // Requested before any window exists, so the surface is composed at the
    // display's real pixel density instead of being scaled up by the system.
    win32::enable_dpi_awareness();
    command::run(env::args().skip(1).collect(), started)
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
