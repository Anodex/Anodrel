//! Fixed UI Automation client verification for the host-owned UI Lab.
//!
//! This diagnostic is intentionally a host route, not a platform service. It
//! opens the compiled UI Lab, reads the provider through Windows on a separate
//! MTA worker, then closes that temporary window. No application can start it,
//! choose what it reads, receive the result, or acquire a UI Automation object.

use std::{
    io,
    sync::mpsc::SyncSender,
    thread,
    time::{Duration, Instant},
};

use anodrel_windows_uia_client::{
    ComApartment, UiAutomationClient, UiAutomationElement, UiAutomationError, UiAutomationNode,
};

use super::{Bool, Hwnd, Lparam, PostMessageW, WM_CLOSE, Wparam};

const UIA_WINDOW_CONTROL_TYPE: i32 = 50_032;
const UIA_TITLE_BAR_CONTROL_TYPE: i32 = 50_037;
const UIA_GROUP_CONTROL_TYPE: i32 = 50_026;
const UIA_TEXT_CONTROL_TYPE: i32 = 50_020;
const UIA_EDIT_CONTROL_TYPE: i32 = 50_004;
const UIA_BUTTON_CONTROL_TYPE: i32 = 50_000;
const ATTACH_RETRY_WINDOW: Duration = Duration::from_secs(2);
const ATTACH_RETRY_INTERVAL: Duration = Duration::from_millis(25);

/// Starts the diagnostic's private MTA worker after its UI Lab is visible.
pub(super) fn spawn(
    window: Hwnd,
    completion: SyncSender<Result<(), UiAutomationError>>,
) -> io::Result<()> {
    thread::Builder::new()
        .name("anodrel-uia-property-probe".to_owned())
        .spawn(move || {
            let outcome = retry_probe(window);
            close_probe_window(window);
            let _ = completion.send(outcome);
        })
        .map(|_| ())
        .map_err(io::Error::other)
}

fn retry_probe(window: Hwnd) -> Result<(), UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match probe_once(window) {
            Ok(()) => return Ok(()),
            Err(
                UiAutomationError::Apartment(_)
                | UiAutomationError::Create(_)
                | UiAutomationError::Query(_)
                | UiAutomationError::NullInterface,
            ) if Instant::now() < deadline => {
                thread::sleep(ATTACH_RETRY_INTERVAL);
            }
            Err(error) => return Err(error),
        }
    }
}

fn probe_once(window: Hwnd) -> Result<(), UiAutomationError> {
    let _apartment = ComApartment::initialize_mta()?;
    let client = UiAutomationClient::connect()?;
    let root = client.element_from_handle(window)?;
    verify_node(&client, &root, ROOT)?;
    Ok(())
}

fn close_probe_window(window: Hwnd) {
    // SAFETY: `window` is the one host-created test window. A posted close
    // carries no pointer or other payload, and failed posting just means the
    // host has already torn the temporary window down.
    let _: Bool = unsafe { PostMessageW(window, WM_CLOSE, Wparam::default(), Lparam::default()) };
}

fn verify_node(
    client: &UiAutomationClient,
    element: &UiAutomationElement,
    expected: ExpectedNode,
) -> Result<(), UiAutomationError> {
    if !(expected.node == client.node(element)?) {
        return Err(UiAutomationError::UnexpectedTree);
    }
    let mut children = client.raw_children(element)?;
    if expected.has_windows_title_bar {
        let Some(title_bar) = children.first() else {
            return Err(UiAutomationError::UnexpectedTree);
        };
        if !(WINDOWS_TITLE_BAR == client.node(title_bar)?) {
            return Err(UiAutomationError::UnexpectedTree);
        }
        children.remove(0);
    }
    if children.len() != expected.children.len() {
        return Err(UiAutomationError::UnexpectedTree);
    }
    for (child, child_expected) in children.iter().zip(expected.children) {
        verify_node(client, child, *child_expected)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ExpectedNode {
    node: ExpectedProperties,
    children: &'static [ExpectedNode],
    has_windows_title_bar: bool,
}

#[derive(Clone, Copy)]
struct ExpectedProperties {
    name: &'static str,
    automation_id: &'static str,
    control_type: i32,
}

impl PartialEq<UiAutomationNode> for ExpectedProperties {
    fn eq(&self, other: &UiAutomationNode) -> bool {
        self.name == other.name
            && self.automation_id == other.automation_id
            && self.control_type == other.control_type
    }
}

const ROOT: ExpectedNode = root(
    "Anodrel UI Lab",
    "anodrel.surface",
    &[VIEWPORT],
    UIA_WINDOW_CONTROL_TYPE,
);
const VIEWPORT: ExpectedNode = group("", "ui.lab.viewport", &[CONTENT], UIA_GROUP_CONTROL_TYPE);
const CONTENT: ExpectedNode = group(
    "",
    "ui.lab.scroll.content",
    &[DOCUMENT, EXERCISES],
    UIA_GROUP_CONTROL_TYPE,
);
const DOCUMENT: ExpectedNode = group(
    "",
    "ui.lab.root",
    &[EYEBROW, TITLE, DETAIL, ACTIONS, STATUS, BOUNDARY],
    UIA_GROUP_CONTROL_TYPE,
);
const EYEBROW: ExpectedNode = text("NATIVE UI FOUNDATION", "ui.lab.eyebrow");
const TITLE: ExpectedNode = text("Anodrel UI Lab", "ui.lab.title");
const DETAIL: ExpectedNode = text(
    "A direct Windows renderer interpreting Anodrel's bounded layout tree.",
    "ui.lab.detail",
);
const ACTIONS: ExpectedNode = group(
    "",
    "ui.lab.actions",
    &[FIELD, INSPECT, HIT_TEST, REPORT],
    UIA_GROUP_CONTROL_TYPE,
);
const FIELD: ExpectedNode = element("Sample field", "ui.lab.field", UIA_EDIT_CONTROL_TYPE, &[]);
const INSPECT: ExpectedNode = button("Inspect layout", "ui.lab.inspect");
const HIT_TEST: ExpectedNode = button("Test semantic action", "ui.lab.hit-test");
const REPORT: ExpectedNode = button("Report semantic action", "ui.lab.report");
const STATUS: ExpectedNode = text("Latest semantic event: none", "ui.lab.status");
const BOUNDARY: ExpectedNode = text(
    "An action reports only its ID. What you type into a field reaches no application at all.",
    "ui.lab.boundary",
);
const EXERCISES: ExpectedNode = group(
    "",
    "ui.lab.scroll.exercises",
    &[
        EXERCISE_1, EXERCISE_2, EXERCISE_3, EXERCISE_4, EXERCISE_5, EXERCISE_6, EXERCISE_7,
        EXERCISE_8, EXERCISE_9,
    ],
    UIA_GROUP_CONTROL_TYPE,
);
const EXERCISE_1: ExpectedNode = button("Scroll exercise 1", "ui.lab.scroll.exercise-1");
const EXERCISE_2: ExpectedNode = button("Scroll exercise 2", "ui.lab.scroll.exercise-2");
const EXERCISE_3: ExpectedNode = button("Scroll exercise 3", "ui.lab.scroll.exercise-3");
const EXERCISE_4: ExpectedNode = button("Scroll exercise 4", "ui.lab.scroll.exercise-4");
const EXERCISE_5: ExpectedNode = button("Scroll exercise 5", "ui.lab.scroll.exercise-5");
const EXERCISE_6: ExpectedNode = button("Scroll exercise 6", "ui.lab.scroll.exercise-6");
const EXERCISE_7: ExpectedNode = button("Scroll exercise 7", "ui.lab.scroll.exercise-7");
const EXERCISE_8: ExpectedNode = button("Scroll exercise 8", "ui.lab.scroll.exercise-8");
const EXERCISE_9: ExpectedNode = button("Scroll exercise 9", "ui.lab.scroll.exercise-9");
const WINDOWS_TITLE_BAR: ExpectedProperties = ExpectedProperties {
    name: "",
    automation_id: "",
    control_type: UIA_TITLE_BAR_CONTROL_TYPE,
};

const fn root(
    name: &'static str,
    automation_id: &'static str,
    children: &'static [ExpectedNode],
    control_type: i32,
) -> ExpectedNode {
    ExpectedNode {
        node: ExpectedProperties {
            name,
            automation_id,
            control_type,
        },
        children,
        has_windows_title_bar: true,
    }
}

const fn group(
    name: &'static str,
    automation_id: &'static str,
    children: &'static [ExpectedNode],
    control_type: i32,
) -> ExpectedNode {
    element(name, automation_id, control_type, children)
}

const fn text(name: &'static str, automation_id: &'static str) -> ExpectedNode {
    element(name, automation_id, UIA_TEXT_CONTROL_TYPE, &[])
}

const fn button(name: &'static str, automation_id: &'static str) -> ExpectedNode {
    element(name, automation_id, UIA_BUTTON_CONTROL_TYPE, &[])
}

const fn element(
    name: &'static str,
    automation_id: &'static str,
    control_type: i32,
    children: &'static [ExpectedNode],
) -> ExpectedNode {
    ExpectedNode {
        node: ExpectedProperties {
            name,
            automation_id,
            control_type,
        },
        children,
        has_windows_title_bar: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_probe_contract_has_a_host_root_and_twenty_three_semantic_nodes() {
        assert_eq!(count_nodes(ROOT), 24);
        assert_eq!(count_nodes(ROOT) - 1, 23);
    }

    fn count_nodes(node: ExpectedNode) -> usize {
        1 + node
            .children
            .iter()
            .copied()
            .map(count_nodes)
            .sum::<usize>()
    }
}
