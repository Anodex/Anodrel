//! Focused verification for the retained Windows UI Lab.

use super::*;

fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

fn id(value: &str) -> ElementId {
    ElementId::new(value).expect("fixed UI Lab ID is valid")
}

/// Tabs until the sample field has focus, then returns the lab.
fn focused_on_the_field() -> UiLab {
    let mut lab = UiLab::new();
    for _ in 0..8 {
        if lab.focus.focused() == Some(&id("ui.lab.field")) {
            return lab;
        }
        lab.focus_next(BASE_WIDTH, BASE_HEIGHT);
    }
    panic!("focus never reached the sample field");
}

mod accessibility;
mod input;
mod presentation;
mod rendering;
mod scrolling;
