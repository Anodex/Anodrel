//! Focused verification for Win32-only helpers and Startup Lab composition.

use super::{
    Body, Canvas, Instant, MIN_CLIENT_HEIGHT, MIN_CLIENT_WIDTH, PackageFacts, PreflightOutcome,
    StartupLab, action_document, document,
    input::mouse_position,
    input::wheel_delta,
    launch::startup_log_book,
    services::{observed_presentation_state, presentation_command},
    startup_lab, window_size_for_client,
};
/// Representative surface state, matching the shipped sample package.
pub(super) fn sample_lab() -> StartupLab {
    StartupLab {
        package: PackageFacts {
            display_name: "Anodrel Sample".to_owned(),
            application_id: "org.anodrel.sample".to_owned(),
            content_format: "anodrel.text.v1".to_owned(),
            content_path: "content/main.txt".to_owned(),
            content_digest: "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"
                .to_owned(),
            content_bytes: 214,
        },
        log: startup_log_book(PreflightOutcome::NotLaunchable.event()),
        startup_millis: 1_240,
        working_set_bytes: 56 * 1024 * 1024,
        last_frame_micros: 3_180,
        revealed_at: Instant::now(),
        hovered: Some(2),
        ambient: false,
        // The shipped default: no machine record has been verified, so the
        // launch tile stays planned.
        launch_available: false,
    }
}

/// Representative surface state for a chosen preflight outcome.
pub(super) fn startup_lab_fixture(launch_available: bool) -> StartupLab {
    StartupLab {
        launch_available,
        ..sample_lab()
    }
}

/// Counts pixels brighter than `threshold` in summed channels.
///
/// A plain "differs from the backdrop" count is useless here: the hero's
/// radial bloom tints nearly every pixel from the first frame. Content —
/// the mark, type, cards — is far brighter than that wash, so a luminance
/// threshold is what actually distinguishes drawn content from background.
fn lit_pixels(canvas: &Canvas, threshold: u16) -> usize {
    (0..canvas.height())
        .flat_map(|y| (0..canvas.width()).map(move |x| (x as i32, y as i32)))
        .filter(|(x, y)| {
            let color = canvas.pixel(*x, *y);
            u16::from(color.red) + u16::from(color.green) + u16::from(color.blue) > threshold
        })
        .count()
}

#[test]
fn a_client_size_grows_into_a_larger_window_size() {
    let (width, height) = window_size_for_client(MIN_CLIENT_WIDTH, MIN_CLIENT_HEIGHT);
    assert!(width >= MIN_CLIENT_WIDTH);
    assert!(height > MIN_CLIENT_HEIGHT, "a title bar should add height");
}

#[test]
fn a_closed_presentation_state_maps_to_only_its_documented_user32_command() {
    assert_eq!(
        presentation_command(anodrel_window::WindowState::Minimized),
        super::SW_MINIMIZE
    );
    assert_eq!(
        presentation_command(anodrel_window::WindowState::Maximized),
        super::SW_MAXIMIZE
    );
    assert_eq!(
        presentation_command(anodrel_window::WindowState::Restored),
        super::SW_RESTORE
    );
}

#[test]
fn a_standard_window_snapshot_reduces_to_the_closed_portable_state() {
    assert_eq!(
        observed_presentation_state(true, true),
        anodrel_window::WindowState::Minimized
    );
    assert_eq!(
        observed_presentation_state(false, true),
        anodrel_window::WindowState::Maximized
    );
    assert_eq!(
        observed_presentation_state(false, false),
        anodrel_window::WindowState::Restored
    );
}

#[test]
fn mouse_coordinates_decode_as_signed_values() {
    assert_eq!(mouse_position(0x0010_0020), (32, 16));
    // A pointer dragged above or left of the client area reports negatives.
    let packed = (((-3_i16) as u16 as u32) << 16 | ((-7_i16) as u16 as u32)) as isize;
    assert_eq!(mouse_position(packed), (-7, -3));
}

#[test]
fn wheel_deltas_decode_as_signed_high_words() {
    assert_eq!(wheel_delta(120usize << 16), 120);
    assert_eq!(wheel_delta(((-120i16 as u16) as usize) << 16), -120);
}

#[test]
fn the_startup_lab_composes_at_every_supported_client_size() {
    let lab = sample_lab();
    for (width, height) in [
        (MIN_CLIENT_WIDTH as u32, MIN_CLIENT_HEIGHT as u32),
        (1_240, 900),
        (2_480, 1_800),
    ] {
        let mut canvas = Canvas::new(width, height);
        startup_lab::draw(&mut canvas, &lab, 99_999);
        let painted = lit_pixels(&canvas, 150);
        assert!(
            painted > (width as usize * height as usize) / 200,
            "{width}x{height} composed only {painted} lit pixels"
        );
    }
}

fn frame_at(elapsed: u64) -> Canvas {
    let mut canvas = Canvas::new(1_240, 900);
    startup_lab::draw(&mut canvas, &sample_lab(), elapsed);
    canvas
}

fn differing_pixels(left: &Canvas, right: &Canvas) -> usize {
    (0..left.height())
        .flat_map(|y| (0..left.width()).map(move |x| (x as i32, y as i32)))
        .filter(|(x, y)| left.pixel(*x, *y) != right.pixel(*x, *y))
        .count()
}

#[test]
fn the_reveal_adds_content_over_time() {
    let opening = lit_pixels(&frame_at(0), 150);
    let midway = lit_pixels(&frame_at(startup_lab::REVEAL_MILLIS / 2), 150);
    let settled = lit_pixels(&frame_at(startup_lab::REVEAL_MILLIS), 150);
    assert!(opening < midway, "the reveal should add content over time");
    assert!(
        midway < settled,
        "the reveal should finish fuller than midway"
    );
}

#[test]
fn everything_but_the_ambient_loop_is_static_once_revealed() {
    // Sampling a whole ambient cycle apart puts the animation at the same
    // phase, so any difference would be a reveal stage still running.
    let settled = frame_at(startup_lab::REVEAL_MILLIS);
    let later = frame_at(startup_lab::REVEAL_MILLIS + startup_lab::ambient::AMBIENT_CYCLE_MILLIS);
    assert_eq!(
        differing_pixels(&settled, &later),
        0,
        "the surface must be identical at equal ambient phase"
    );
}

#[test]
fn ambient_motion_actually_moves() {
    // Mid-sweep against a point in the cycle with no sweep at all.
    let swept =
        frame_at(startup_lab::REVEAL_MILLIS + startup_lab::ambient::AMBIENT_CYCLE_MILLIS / 10);
    let quiet =
        frame_at(startup_lab::REVEAL_MILLIS + startup_lab::ambient::AMBIENT_CYCLE_MILLIS / 2);
    assert!(
        differing_pixels(&swept, &quiet) > 5_000,
        "the mark should visibly change across the ambient cycle"
    );
}

#[test]
fn a_partial_ambient_frame_reproduces_a_full_one() {
    // The partial path restores the backdrop and recomposites cached
    // layers. If it ever diverged from a full compose, the mark's region
    // would drift out of step with the rest of the surface.
    let elapsed = startup_lab::REVEAL_MILLIS + 900;
    let full = frame_at(elapsed);
    let mut partial = frame_at(elapsed);
    assert!(
        startup_lab::draw_ambient(&mut partial, &sample_lab(), elapsed),
        "the ambient path should be available once settled"
    );
    assert_eq!(
        differing_pixels(&full, &partial),
        0,
        "a partial update must match a full compose exactly"
    );
}

#[test]
fn ambient_motion_stays_inside_its_declared_region() {
    // Whatever moves must be inside the region the host invalidates, or
    // the screen would tear where an update was never sent.
    let region = startup_lab::ambient_region(1_240.0, 900.0).expect("region available");
    let swept =
        frame_at(startup_lab::REVEAL_MILLIS + startup_lab::ambient::AMBIENT_CYCLE_MILLIS / 10);
    let quiet =
        frame_at(startup_lab::REVEAL_MILLIS + startup_lab::ambient::AMBIENT_CYCLE_MILLIS / 2);
    for y in 0..900_i32 {
        for x in 0..1_240_i32 {
            if swept.pixel(x, y) == quiet.pixel(x, y) {
                continue;
            }
            assert!(
                (x as f32) >= region.left.floor()
                    && (x as f32) < region.right.ceil()
                    && (y as f32) >= region.top.floor()
                    && (y as f32) < region.bottom.ceil(),
                "pixel ({x}, {y}) changes outside the ambient region {region:?}"
            );
        }
    }
}

#[test]
fn linked_actions_produce_a_document_and_planned_actions_do_not() {
    let lab = sample_lab();
    for action in &startup_lab::ACTIONS {
        let produced = action_document(action.kind, &lab);
        assert_eq!(
            produced.is_some(),
            action.linked,
            "{:?} disagrees with its linked state",
            action.kind
        );
    }
}

#[test]
fn a_panicking_window_message_is_contained_rather_than_aborting() {
    // `window_proc` is `extern "system"`, which does not unwind, so an
    // escaping panic aborts the process and runs no destructor. That would
    // strand a verified product child with no host to shut it down.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let contained = super::contain_panic(|| panic!("a paint failure must not abort the host"));
    std::panic::set_hook(previous);

    assert_eq!(contained, None);
    assert_eq!(super::contain_panic(|| 42), Some(42));
}

#[test]
fn a_crash_record_names_the_kind_of_surface_and_nothing_else() {
    use anodrel_crash::CrashSurface;

    let _exclusive = super::registry::tests_exclusive();
    // An unregistered window first: a panic can arrive before a view is
    // registered or after it is gone, and that must classify rather than
    // fail. The record path exists to leave evidence, so it has no branch
    // that gives up.
    assert_eq!(super::registry::crash_surface(-950), CrashSurface::Unknown);

    super::registry::insert(
        -951,
        super::View::Document(document::Document::from_text(
            "a title nothing may record",
            "test",
            "body",
        )),
    )
    .expect("view registers");
    super::registry::insert(-952, super::View::StartupLab(sample_lab())).expect("view registers");

    assert_eq!(super::registry::crash_surface(-951), CrashSurface::Document);
    assert_eq!(
        super::registry::crash_surface(-952),
        CrashSurface::StartupLab
    );

    // The whole catalogue is plain labels. A surface that carried a title,
    // an application identity, or a handle would put unbounded text into a
    // file this platform promises holds none.
    for surface in CrashSurface::ALL {
        assert!(
            surface
                .label()
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        );
    }

    super::registry::clear().expect("registry clears");
}

#[test]
fn clearing_the_registry_drops_views_the_loop_left_behind() {
    // A contained panic ends the message loop while windows are still
    // registered, so the host clears them itself.
    let _exclusive = super::registry::tests_exclusive();
    super::registry::insert(
        -901,
        super::View::Document(document::Document::from_text("stranded", "test", "body")),
    )
    .expect("view registers");

    assert_eq!(super::registry::clear().expect("registry clears"), 1);
    assert_eq!(super::registry::clear().expect("registry clears again"), 0);
}

#[test]
fn log_document_contains_only_the_fixed_startup_catalogue() {
    let lab = sample_lab();
    let Some((_, document)) = action_document(startup_lab::ActionKind::OpenLogs, &lab) else {
        panic!("the linked log action needs its host document");
    };
    let Body::Sections(sections) = document.body else {
        panic!("the log document must be structured");
    };
    let events = &sections[0].rows;
    assert_eq!(events.len(), 5);
    assert_eq!(events[0].0, "#0001");
    assert_eq!(events[4].0, "#0005");
    // The launch preflight sits between the transport check and the
    // surface being authorized, and reports only that the host ran it.
    assert!(events[3].1.contains("launch"));
    assert!(events[3].1.contains("Verified launch preflight completed."));
    for (_, reading) in events {
        assert!(!reading.contains(char::from(92)));
        assert!(!reading.contains('/'));
        assert!(!reading.contains(':'));
    }
}

#[test]
fn an_action_document_never_carries_a_filesystem_path() {
    let lab = sample_lab();
    for action in &startup_lab::ACTIONS {
        let Some((_, document)) = action_document(action.kind, &lab) else {
            continue;
        };
        let mut canvas = Canvas::new(760, 560);
        document::draw(&mut canvas, &document);
        assert!(lit_pixels(&canvas, 150) > 1_000, "document drew nothing");
    }
    // The window layer only ever holds the manifest-relative path.
    assert!(!lab.package.content_path.contains(':'));
    assert!(!lab.package.content_path.contains('\\'));
}
