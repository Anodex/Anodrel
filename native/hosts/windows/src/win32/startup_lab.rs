//! Branded, host-owned layout for the Anodrel Startup Lab.

use super::{
    Hdc, Rect, StartupLab,
    paint::{
        CYAN, HEADER, INK, MIDNIGHT, MUTED, PANEL, PANEL_BORDER, READY, VIOLET, fill, label, line,
        outline_ellipse, solid_ellipse,
    },
};

const FW_SEMIBOLD: i32 = 600;
const FW_NORMAL: i32 = 400;

pub(super) fn draw(device_context: Hdc, client: Rect, lab: &StartupLab) {
    let width = client.width().max(1);
    let height = client.height().max(1);
    let margin = (width / 24).clamp(32, 64);

    fill(device_context, client, MIDNIGHT);
    fill(device_context, Rect::new(0, 0, width, 76), HEADER);
    line(device_context, 0, 75, width, 75, 1, PANEL_BORDER);

    label(
        device_context,
        Rect::new(margin, 23, margin + 260, 54),
        "ANODREL",
        22,
        FW_SEMIBOLD,
        CYAN,
        true,
    );
    label(
        device_context,
        Rect::new(margin + 134, 29, margin + 530, 54),
        "NATIVE APPLICATION PLATFORM",
        12,
        FW_SEMIBOLD,
        MUTED,
        true,
    );
    label(
        device_context,
        Rect::new(width - margin - 255, 30, width - margin, 54),
        "WINDOWS FOUNDATION / STARTUP LAB",
        11,
        FW_SEMIBOLD,
        VIOLET,
        true,
    );

    let mark_x = width / 2;
    let mark_y = 244;
    draw_orbital_mark(device_context, mark_x, mark_y);

    label(
        device_context,
        Rect::new(mark_x - 220, 355, mark_x + 220, 388),
        &lab.display_name,
        24,
        FW_SEMIBOLD,
        INK,
        true,
    );
    let identity = format!("VALIDATED APPLICATION  /  {}", lab.application_id);
    label(
        device_context,
        Rect::new(mark_x - 260, 394, mark_x + 260, 416),
        &identity,
        11,
        FW_SEMIBOLD,
        MUTED,
        true,
    );

    let cards_top = (height - 188).max(458);
    let gap = 16;
    let cards_width = width - margin * 2;
    let card_width = ((cards_width - gap * 2) / 3).max(220);
    let card_height = 122;
    draw_card(
        device_context,
        Rect::new(
            margin,
            cards_top,
            margin + card_width,
            cards_top + card_height,
        ),
        CYAN,
        "OWNED CORE",
        "platform.health: ready",
        "strict JSON + policy core",
    );
    draw_card(
        device_context,
        Rect::new(
            margin + card_width + gap,
            cards_top,
            margin + card_width * 2 + gap,
            cards_top + card_height,
        ),
        READY,
        "VERIFIED PACKAGE",
        "manifest + digest: ready",
        "contained local content",
    );
    draw_card(
        device_context,
        Rect::new(
            margin + (card_width + gap) * 2,
            cards_top,
            width - margin,
            cards_top + card_height,
        ),
        VIOLET,
        "PRIVATE IPC",
        "loopback self-test: ready",
        "auth + health round trip",
    );

    let footer_top = cards_top + card_height + 24;
    label(
        device_context,
        Rect::new(margin, footer_top, width - margin, footer_top + 22),
        "START.BAT  ->  PACKAGE + CORE + IPC CHECK  ->  DIRECT WIN32 STARTUP LAB",
        11,
        FW_SEMIBOLD,
        MUTED,
        true,
    );
}

fn draw_orbital_mark(device_context: Hdc, center_x: i32, center_y: i32) {
    outline_ellipse(
        device_context,
        Rect::new(center_x - 94, center_y - 94, center_x + 94, center_y + 94),
        3,
        CYAN,
    );
    outline_ellipse(
        device_context,
        Rect::new(center_x - 142, center_y - 47, center_x + 142, center_y + 47),
        3,
        VIOLET,
    );
    line(
        device_context,
        center_x - 66,
        center_y + 64,
        center_x,
        center_y - 86,
        4,
        CYAN,
    );
    line(
        device_context,
        center_x,
        center_y - 86,
        center_x + 66,
        center_y + 64,
        4,
        CYAN,
    );
    line(
        device_context,
        center_x - 38,
        center_y + 8,
        center_x + 38,
        center_y + 8,
        4,
        VIOLET,
    );
    line(
        device_context,
        center_x - 112,
        center_y + 26,
        center_x - 38,
        center_y + 8,
        2,
        VIOLET,
    );
    line(
        device_context,
        center_x + 112,
        center_y + 26,
        center_x + 38,
        center_y + 8,
        2,
        VIOLET,
    );
    solid_ellipse(
        device_context,
        Rect::new(center_x - 10, center_y - 10, center_x + 10, center_y + 10),
        READY,
    );
    for (x, y, color) in [
        (center_x, center_y - 94, CYAN),
        (center_x - 122, center_y + 26, VIOLET),
        (center_x + 122, center_y + 26, VIOLET),
    ] {
        solid_ellipse(
            device_context,
            Rect::new(x - 11, y - 11, x + 11, y + 11),
            color,
        );
    }
}

fn draw_card(
    device_context: Hdc,
    rect: Rect,
    accent: u32,
    title: &str,
    status: &str,
    detail: &str,
) {
    fill(device_context, rect, PANEL);
    line(
        device_context,
        rect.left,
        rect.top,
        rect.right,
        rect.top,
        3,
        accent,
    );
    solid_ellipse(
        device_context,
        Rect::new(rect.left + 18, rect.top + 22, rect.left + 29, rect.top + 33),
        accent,
    );
    label(
        device_context,
        Rect::new(
            rect.left + 38,
            rect.top + 17,
            rect.right - 18,
            rect.top + 39,
        ),
        title,
        12,
        FW_SEMIBOLD,
        INK,
        true,
    );
    label(
        device_context,
        Rect::new(
            rect.left + 18,
            rect.top + 55,
            rect.right - 18,
            rect.top + 79,
        ),
        status,
        15,
        FW_SEMIBOLD,
        accent,
        true,
    );
    label(
        device_context,
        Rect::new(
            rect.left + 18,
            rect.top + 87,
            rect.right - 18,
            rect.top + 108,
        ),
        detail,
        11,
        FW_NORMAL,
        MUTED,
        true,
    );
}
