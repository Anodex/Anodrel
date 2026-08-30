#![forbid(unsafe_code)]

//! First-party scaffolding for the constrained Anodrel native UI template.
//!
//! The tool writes a new Rust project only. It neither runs the project nor
//! changes machine policy, signing, installation, or host capabilities.

mod arguments;
mod init;
mod paths;
mod template;
mod validation;

use std::env;

use arguments::{TemplateKind, parse};
use init::{
    initialize, initialize_context_menu, initialize_form, initialize_live_status, initialize_menu,
    initialize_multi_window, initialize_scroll_window, initialize_window_controls,
};

const USAGE: &str = concat!(
    "usage:\n",
    "  anodrel-native-app-tool init <destination> <project-slug> <display-label>\n",
    "  anodrel-native-app-tool init-form <destination> <project-slug> <display-label>\n",
    "  anodrel-native-app-tool init-live-status <destination> <project-slug> <display-label>\n",
    "  anodrel-native-app-tool init-menu <destination> <project-slug> <display-label>\n",
    "  anodrel-native-app-tool init-context-menu <destination> <project-slug> <display-label>\n",
    "  anodrel-native-app-tool init-multi-window <destination> <project-slug> <display-label>\n",
    "  anodrel-native-app-tool init-scroll-window <destination> <project-slug> <display-label>\n",
    "  anodrel-native-app-tool init-window-controls <destination> <project-slug> <display-label>"
);

fn main() {
    let result = match parse(env::args().skip(1)) {
        Ok(command) => match command.template_kind {
            TemplateKind::Ui => initialize(
                &command.destination,
                &command.project_slug,
                &command.display_label,
            ),
            TemplateKind::Form => initialize_form(
                &command.destination,
                &command.project_slug,
                &command.display_label,
            ),
            TemplateKind::LiveStatus => initialize_live_status(
                &command.destination,
                &command.project_slug,
                &command.display_label,
            ),
            TemplateKind::Menu => initialize_menu(
                &command.destination,
                &command.project_slug,
                &command.display_label,
            ),
            TemplateKind::ContextMenu => initialize_context_menu(
                &command.destination,
                &command.project_slug,
                &command.display_label,
            ),
            TemplateKind::MultiWindow => initialize_multi_window(
                &command.destination,
                &command.project_slug,
                &command.display_label,
            ),
            TemplateKind::ScrollWindow => initialize_scroll_window(
                &command.destination,
                &command.project_slug,
                &command.display_label,
            ),
            TemplateKind::WindowControls => initialize_window_controls(
                &command.destination,
                &command.project_slug,
                &command.display_label,
            ),
        }
        .map_err(|error| error.to_string()),
        Err(()) => Err(USAGE.to_owned()),
    };
    if let Err(error) = result {
        eprintln!("anodrel-native-app-tool: {error}");
        std::process::exit(2);
    }
}
