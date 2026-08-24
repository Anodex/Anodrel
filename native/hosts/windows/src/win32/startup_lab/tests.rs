//! Focused verification for Startup Lab layout, motion, and paint.

use super::{
    ACTIONS, ActionKind, BASE_HEIGHT, BASE_WIDTH, CARDS, Layout, TextSpec, WEIGHT_MEDIUM,
    WEIGHT_REGULAR, action_at, planned_marker, stage, tile_is_live, tile_marker, tile_subtitle,
    tile_text_rows,
};
use anodrel_canvas::point;

#[test]
fn the_layout_keeps_every_region_inside_the_client_area() {
    for (width, height) in [(BASE_WIDTH, BASE_HEIGHT), (900.0, 660.0), (2400.0, 1500.0)] {
        let layout = Layout::new(width, height);
        assert!(
            layout.mark.top >= layout.header_height,
            "mark overlaps header"
        );
        for index in 0..CARDS.len() {
            let rect = layout.card_rect(index);
            assert!(
                rect.left >= 0.0 && rect.right <= width,
                "card {index} overflows"
            );
        }
        assert!(layout.actions.right <= width, "action strip overflows");
        assert!(layout.footer_top < height, "footer falls off the surface");
    }
}

#[test]
fn the_card_status_line_clears_its_badge() {
    // The badge and the status line share a left edge, so any vertical
    // overlap draws the circle's arc straight through the text. This was
    // shipped once: the badge ran to `top + 72` while the status line
    // started at `top + 62`.
    for (width, height) in [
        (BASE_WIDTH, BASE_HEIGHT),
        (900.0, 660.0),
        (2_480.0, 1_800.0),
    ] {
        let layout = Layout::new(width, height);
        for index in 0..CARDS.len() {
            let rect = layout.card_rect(index);
            let badge = super::card_badge(&layout, rect);
            let status_top = super::card_status_top(&layout, rect);
            assert!(
                status_top >= badge.bottom,
                "card {index} at {width}x{height}: status starts at {status_top} \
                 but the badge runs to {}",
                badge.bottom
            );
        }
    }
}

#[test]
fn every_card_element_stays_inside_its_card() {
    let layout = Layout::new(BASE_WIDTH, BASE_HEIGHT);
    for index in 0..CARDS.len() {
        let rect = layout.card_rect(index);
        let badge = super::card_badge(&layout, rect);
        assert!(badge.left >= rect.left && badge.right <= rect.right);
        assert!(badge.top >= rect.top && badge.bottom <= rect.bottom);
        assert!(
            super::card_status_top(&layout, rect) < rect.bottom,
            "card {index}: the status line falls outside the card"
        );
    }
}

#[test]
fn the_hero_mark_is_square() {
    // The authored asset is a square crop and the geometry fills the unit
    // square, so a non-square target stretches the logo.
    for (width, height) in [(BASE_WIDTH, BASE_HEIGHT), (1_600.0, 1_000.0)] {
        let mark = Layout::new(width, height).mark;
        assert!(
            (mark.width() - mark.height()).abs() < 0.01,
            "the mark is {}x{} at {width}x{height}",
            mark.width(),
            mark.height()
        );
    }
}

#[test]
fn cards_are_ordered_left_to_right_without_overlapping() {
    let layout = Layout::new(BASE_WIDTH, BASE_HEIGHT);
    for index in 1..CARDS.len() {
        let previous = layout.card_rect(index - 1);
        let current = layout.card_rect(index);
        assert!(previous.right <= current.left, "cards {index} overlap");
    }
}

#[test]
fn every_action_tile_is_hit_testable_at_its_own_centre() {
    let layout = Layout::new(BASE_WIDTH, BASE_HEIGHT);
    for index in 0..ACTIONS.len() {
        let center = layout.action_rect(index).center();
        assert_eq!(
            action_at(BASE_WIDTH, BASE_HEIGHT, center),
            Some(index),
            "action {index} is not hit-testable"
        );
    }
}

#[test]
fn points_outside_the_action_strip_hit_nothing() {
    assert_eq!(action_at(BASE_WIDTH, BASE_HEIGHT, point(4.0, 4.0)), None);
    assert_eq!(
        action_at(
            BASE_WIDTH,
            BASE_HEIGHT,
            point(BASE_WIDTH / 2.0, BASE_HEIGHT - 4.0)
        ),
        None
    );
}

#[test]
fn hit_testing_follows_the_layout_when_the_window_is_resized() {
    let width = 1600.0;
    let height = 1000.0;
    let layout = Layout::new(width, height);
    let center = layout.action_rect(2).center();
    assert_eq!(action_at(width, height, center), Some(2));
}

#[test]
fn a_stage_runs_from_zero_to_one_and_holds() {
    assert_eq!(stage(0, 100.0, 200.0), 0.0);
    assert!(stage(200, 100.0, 200.0) > 0.0);
    assert!(stage(200, 100.0, 200.0) < 1.0);
    assert_eq!(stage(300, 100.0, 200.0), 1.0);
    assert_eq!(stage(9_999, 100.0, 200.0), 1.0);
}

#[test]
fn only_actions_backed_by_a_host_operation_are_linked() {
    for action in &ACTIONS {
        let expected = matches!(
            action.kind,
            ActionKind::OpenLogs | ActionKind::InspectPackage | ActionKind::RuntimeDiagnostics
        );
        assert_eq!(
            action.linked, expected,
            "{:?} is linked without a documented host operation behind it",
            action.kind
        );
    }
}

#[test]
fn the_launch_tile_is_inert_until_a_preflight_says_the_fixture_validated() {
    let unprovisioned = super::super::tests::startup_lab_fixture(false);
    let provisioned = super::super::tests::startup_lab_fixture(true);

    for action in &ACTIONS {
        let live_when_unprovisioned = tile_is_live(action, &unprovisioned);
        if action.kind == ActionKind::LaunchDevelopmentFixture {
            assert!(
                !live_when_unprovisioned,
                "the launch tile is live without a validated fixture"
            );
            assert!(
                tile_is_live(action, &provisioned),
                "the launch tile stays inert after a successful preflight"
            );
        } else {
            // Every other tile displays values the host already held, so
            // machine provisioning must not change its availability.
            assert_eq!(live_when_unprovisioned, action.linked);
            assert_eq!(tile_is_live(action, &provisioned), action.linked);
        }
    }
}

#[test]
fn every_tile_label_fits_its_slot_at_the_smallest_supported_size() {
    // Tile text is drawn from the badge's right edge towards the slot's
    // right edge and is never wrapped or ellipsized, so a label that
    // overruns is simply painted over whatever is there — the chevron on a
    // live tile, the planned marker on one that is not.
    let unprovisioned = super::super::tests::startup_lab_fixture(false);
    let provisioned = super::super::tests::startup_lab_fixture(true);

    for (width, height) in [(900.0, 660.0), (BASE_WIDTH, BASE_HEIGHT)] {
        let layout = Layout::new(width, height);
        for (index, action) in ACTIONS.iter().enumerate() {
            let slot = layout.action_rect(index);
            // Mirrors the badge geometry the drawing code uses.
            let text_left = slot.left + layout.unit(22.0) + layout.unit(44.0) + layout.unit(15.0);

            // Both states of every tile: a tile that is planned on this
            // machine has far less room than the same tile when it is live.
            for lab in [&unprovisioned, &provisioned] {
                let marker = tile_marker(&layout, slot, tile_is_live(action, lab));
                for (label, size, weight, limit) in [
                    (action.title, 16.0, WEIGHT_MEDIUM, marker.title_limit),
                    (
                        tile_subtitle(action, lab),
                        12.0,
                        WEIGHT_REGULAR,
                        marker.subtitle_limit,
                    ),
                ] {
                    let available = limit - text_left;
                    let measured =
                        super::text::width(&TextSpec::new(label, layout.font(size), weight));
                    assert!(
                        measured <= available,
                        "{label:?} needs {measured} of {available} at {width}x{height}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_planned_marker_clears_the_title_it_sits_below() {
    // The marker used to be centred on the slot, where a long title ran
    // straight through it. It now shares the subtitle's line, so what has
    // to hold is that the line it moved to is genuinely clear of the one
    // above: horizontal room alone would not have caught the original
    // fault either.
    for (width, height) in [(900.0, 660.0), (BASE_WIDTH, BASE_HEIGHT)] {
        let layout = Layout::new(width, height);
        let slot = layout.action_rect(0);
        let (title_top, subtitle_top) = tile_text_rows(&layout, slot);
        let title = TextSpec::new("Development Fixture", layout.font(16.0), WEIGHT_MEDIUM);
        let marker = planned_marker(&layout);
        let title_bottom = title_top + super::text::line_height(&title);
        assert!(
            subtitle_top >= title_bottom,
            "the marker's line starts at {subtitle_top}, inside a title ending at {title_bottom}"
        );
        assert!(
            super::text::line_height(&marker) <= super::text::line_height(&title),
            "the marker is taller than the line it was moved out of"
        );
    }
}

#[test]
fn the_launch_tile_names_itself_a_development_fixture_in_both_states() {
    // The tile must never read as a product launch. The word "development"
    // in its title is what stops it being mistaken for one.
    let launch = ACTIONS
        .iter()
        .find(|action| action.kind == ActionKind::LaunchDevelopmentFixture)
        .expect("the launch tile exists");
    assert!(launch.title.contains("Development"));

    let unprovisioned = super::super::tests::startup_lab_fixture(false);
    assert_eq!(tile_subtitle(launch, &unprovisioned), "Not provisioned");

    let provisioned = super::super::tests::startup_lab_fixture(true);
    let live = tile_subtitle(launch, &provisioned);
    assert_eq!(live, "Development only, not a product");
    // A live tile must not claim more than it is. "Verified" beside a
    // launch control invites reading a fixture as a product.
    assert!(!live.to_ascii_lowercase().contains("verified"));
}
