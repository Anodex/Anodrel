//! The host-owned document surface.
//!
//! Every window that is not the Startup Lab renders through here: the validated
//! package text, the window-lifecycle diagnostics, and the views opened from the
//! Startup Lab's action strip. One surface means a second window looks like part
//! of the same product rather than a debug console.
//!
//! Content is laid out, never interpreted. Application text arrives as opaque
//! paragraphs and is measured and wrapped, with no markup, link, or script
//! meaning attached to any character.

use anodrel_brand::{mark, mark::MarkStyle, palette};
use anodrel_canvas::{Canvas, Paint, Rect, Stop, point};

use super::text;
use super::text::{Align, TextSpec};

const WEIGHT_REGULAR: i32 = 400;
const WEIGHT_MEDIUM: i32 = 500;
const WEIGHT_SEMIBOLD: i32 = 600;

/// A labelled group of key/value rows.
#[derive(Clone)]
pub(super) struct Section {
    /// Small-caps heading above the rows.
    pub(super) heading: String,
    /// Label and value pairs, rendered as an aligned two-column list.
    pub(super) rows: Vec<(String, String)>,
}

/// What a document displays below its header.
#[derive(Clone)]
pub(super) enum Body {
    /// Free text, wrapped to the content column.
    Paragraphs(Vec<String>),
    /// Structured readings.
    Sections(Vec<Section>),
}

/// A host-owned informational window.
#[derive(Clone)]
pub(super) struct Document {
    /// Headline, shown beside the mark.
    pub(super) title: String,
    /// One line of context below the headline.
    pub(super) subtitle: String,
    /// The document's content.
    pub(super) body: Body,
}

impl Document {
    /// Builds a document from free text, splitting on blank lines.
    pub(super) fn from_text(title: &str, subtitle: &str, text: &str) -> Self {
        let paragraphs = text
            .split("\n\n")
            .map(|paragraph| paragraph.replace('\n', " ").trim().to_owned())
            .filter(|paragraph| !paragraph.is_empty())
            .collect();
        Self {
            title: title.to_owned(),
            subtitle: subtitle.to_owned(),
            body: Body::Paragraphs(paragraphs),
        }
    }
}

/// Draws a document into the full client area.
pub(super) fn draw(canvas: &mut Canvas, document: &Document) {
    let width = canvas.width() as f32;
    let height = canvas.height() as f32;
    let scale = (width / 760.0).clamp(0.75, 2.5);
    let unit = |value: f32| value * scale;
    let font = |value: f32| (value * scale).round().max(1.0) as i32;
    let margin = unit(38.0);

    canvas.clear(palette::BACKDROP);
    canvas.fill_rect(
        canvas.bounds(),
        &Paint::linear(
            point(0.0, 0.0),
            point(0.0, height),
            vec![
                Stop::new(0.0, palette::BACKDROP_LIFT.with_alpha(150)),
                Stop::new(1.0, palette::BACKDROP.with_alpha(0)),
            ],
        ),
    );

    let mark_size = unit(42.0);
    mark::draw(
        canvas,
        Rect::from_size(margin, unit(30.0), mark_size, mark_size),
        MarkStyle::compact(),
    );

    let title = TextSpec::new(&document.title, font(24.0), WEIGHT_MEDIUM);
    text::draw(
        canvas,
        &title,
        point(margin + mark_size + unit(18.0), unit(30.0)),
        Align::Left,
        &Paint::solid(palette::INK),
    );
    let subtitle = TextSpec::new(&document.subtitle, font(13.0), WEIGHT_REGULAR);
    text::draw(
        canvas,
        &subtitle,
        point(margin + mark_size + unit(18.0), unit(58.0)),
        Align::Left,
        &Paint::solid(palette::INK_MUTED),
    );

    let mut cursor = unit(96.0);
    canvas.fill_rect(
        Rect::new(margin, cursor, width - margin, cursor + unit(1.0)),
        &Paint::solid(palette::PANEL_EDGE),
    );
    cursor += unit(26.0);

    let content_width = width - margin * 2.0;
    match &document.body {
        Body::Paragraphs(paragraphs) => {
            for paragraph in paragraphs {
                let spec = TextSpec::new(paragraph.clone(), font(15.0), WEIGHT_REGULAR);
                let line_height = text::line_height(&spec) * 1.45;
                for line in wrap(paragraph, font(15.0), WEIGHT_REGULAR, content_width) {
                    if cursor + line_height > height - unit(20.0) {
                        return;
                    }
                    text::draw(
                        canvas,
                        &TextSpec::new(line, font(15.0), WEIGHT_REGULAR),
                        point(margin, cursor),
                        Align::Left,
                        &Paint::solid(palette::INK_SOFT),
                    );
                    cursor += line_height;
                }
                cursor += unit(12.0);
            }
        }
        Body::Sections(sections) => {
            let label_column = margin + unit(4.0);
            let value_column = margin + (content_width * 0.36).min(unit(240.0));
            for section in sections {
                if cursor > height - unit(60.0) {
                    return;
                }
                let heading = TextSpec::new(section.heading.clone(), font(11.0), WEIGHT_SEMIBOLD)
                    .tracked(unit(1.0).round() as i32);
                text::draw(
                    canvas,
                    &heading,
                    point(label_column, cursor),
                    Align::Left,
                    &Paint::solid(palette::ACCENT_SHELL),
                );
                cursor += unit(24.0);

                for (label, value) in &section.rows {
                    if cursor > height - unit(28.0) {
                        return;
                    }
                    text::draw(
                        canvas,
                        &TextSpec::new(label.clone(), font(14.0), WEIGHT_REGULAR),
                        point(label_column, cursor),
                        Align::Left,
                        &Paint::solid(palette::INK_MUTED),
                    );
                    // Long readings such as a digest are wrapped rather than
                    // clipped, so a value is never silently truncated.
                    for line in wrap(
                        value,
                        font(14.0),
                        WEIGHT_MEDIUM,
                        width - margin - value_column,
                    ) {
                        text::draw(
                            canvas,
                            &TextSpec::new(line, font(14.0), WEIGHT_MEDIUM),
                            point(value_column, cursor),
                            Align::Left,
                            &Paint::solid(palette::INK),
                        );
                        cursor += unit(22.0);
                    }
                }
                cursor += unit(18.0);
            }
        }
    }
}

/// Greedily wraps text to a pixel width, breaking an over-long word by character.
fn wrap(text_value: &str, size: i32, weight: i32, available: f32) -> Vec<String> {
    if available <= 0.0 {
        return Vec::new();
    }
    let fits = |candidate: &str| {
        text::width(&TextSpec::new(candidate.to_owned(), size, weight)) <= available
    };
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text_value.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if fits(&candidate) {
            current = candidate;
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if fits(word) {
            current = word.to_owned();
            continue;
        }
        // A single word wider than the column, such as a hex digest.
        let mut chunk = String::new();
        for character in word.chars() {
            let mut candidate = chunk.clone();
            candidate.push(character);
            if fits(&candidate) {
                chunk = candidate;
            } else {
                lines.push(std::mem::take(&mut chunk));
                chunk.push(character);
            }
        }
        current = chunk;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{Body, Document, Section, draw, wrap};
    use anodrel_brand::palette;
    use anodrel_canvas::Canvas;

    #[test]
    fn text_splits_into_paragraphs_on_blank_lines() {
        let document = Document::from_text("T", "S", "first line\nstill first\n\nsecond");
        let Body::Paragraphs(paragraphs) = document.body else {
            panic!("expected paragraphs");
        };
        assert_eq!(paragraphs, vec!["first line still first", "second"]);
    }

    #[test]
    fn wrapping_respects_the_available_width() {
        let lines = wrap(
            "the quick brown fox jumps over the lazy dog",
            14,
            400,
            120.0,
        );
        assert!(lines.len() > 1, "expected the text to wrap");
        for line in &lines {
            assert!(!line.is_empty());
        }
    }

    #[test]
    fn an_over_long_word_is_broken_rather_than_clipped() {
        let digest = "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585";
        let lines = wrap(digest, 14, 500, 100.0);
        assert!(lines.len() > 1, "a long digest should be split");
        let rejoined: String = lines.concat();
        assert_eq!(rejoined, digest, "splitting must not lose characters");
    }

    #[test]
    fn wrapping_into_no_space_yields_nothing_instead_of_looping() {
        assert!(wrap("anything at all", 14, 400, 0.0).is_empty());
    }

    #[test]
    fn a_paragraph_document_paints_its_header_and_body() {
        let mut canvas = Canvas::new(760, 460);
        draw(
            &mut canvas,
            &Document::from_text("Anodrel", "Owned surface", "A body paragraph to render."),
        );
        let lit = (0..460)
            .flat_map(|y| (0..760).map(move |x| (x, y)))
            .filter(|(x, y)| canvas.pixel(*x, *y) != palette::BACKDROP)
            .count();
        assert!(lit > 2_000, "expected a drawn surface, found {lit} pixels");
    }

    #[test]
    fn a_section_document_renders_without_overflowing_a_short_window() {
        let mut canvas = Canvas::new(600, 200);
        draw(
            &mut canvas,
            &Document {
                title: "Inspect".to_owned(),
                subtitle: "Verified package".to_owned(),
                body: Body::Sections(vec![Section {
                    heading: "CONTENT".to_owned(),
                    rows: (0..40)
                        .map(|index| (format!("row {index}"), format!("value {index}")))
                        .collect(),
                }]),
            },
        );
        assert_eq!(canvas.width(), 600);
    }
}
