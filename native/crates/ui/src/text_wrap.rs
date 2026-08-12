//! Breaking one validated text value into the visual lines that fit a width.
//!
//! A text node's *value* is a single line: control characters, newlines
//! included, are refused by the model. This module is about **presentation**,
//! not content — the host decides how many visual lines that one value occupies
//! at the width it has. An application supplies the words and never learns
//! where they broke.
//!
//! The breaking lives here, in the portable crate, rather than in a host, for
//! two reasons. It can be tested against a predictable measurer instead of a
//! real font, and layout and drawing can call the same function, so a wrapped
//! paragraph's measured height and its painted lines cannot drift apart.
//!
//! See `docs/UI.md` and Decision 0068.

use crate::TextMeasurer;

/// The most visual lines one text value may occupy.
///
/// A value is already bounded by the model, so this is not what stops a large
/// document. It bounds the *other* direction: a long value in a narrow column
/// could otherwise produce a line for nearly every character and a surface tall
/// enough to stall a paint. Text past this limit stays on the final line and is
/// clipped by the client rectangle, which is the behaviour every text value had
/// before wrapping existed.
pub const MAX_TEXT_LINES: usize = 64;

/// Breaks `value` into the visual lines that fit within `max_width`.
///
/// Returns borrowed slices of `value` in order, always at least one, so an
/// empty value still occupies one line's height rather than collapsing.
///
/// Breaking is greedy and happens at spaces. A single word wider than the
/// available width is broken between characters instead, because the
/// alternative — leaving it whole — is the clipping this exists to remove. A
/// non-finite or non-positive width cannot be wrapped against, so the value is
/// returned as one line.
///
/// Candidate lines are measured whole, never by summing word widths, because
/// shaping means the width of `"a b"` is not obliged to equal the width of
/// `"a"` plus the width of `" b"`.
#[must_use]
pub fn wrap_text<'a>(
    value: &'a str,
    font_size: u16,
    max_width: f32,
    measurer: &dyn TextMeasurer,
) -> Vec<&'a str> {
    if value.is_empty() {
        return vec![""];
    }
    if !max_width.is_finite() || max_width <= 0.0 {
        return vec![value];
    }
    if fits(value, font_size, max_width, measurer) {
        return vec![value];
    }

    let mut lines = Vec::new();
    let mut line_start = 0;
    // The end of the last candidate known to fit. Zero means nothing on this
    // line fits yet, which is what sends a long word to the character breaker.
    let mut committed = 0;

    for boundary in break_candidates(value) {
        if boundary <= line_start {
            continue;
        }
        if fits(&value[line_start..boundary], font_size, max_width, measurer) {
            committed = boundary;
            continue;
        }

        // The words up to this boundary no longer fit, so end the line at the
        // last boundary that did.
        if committed > line_start {
            lines.push(value[line_start..committed].trim_end());
            line_start = skip_space(value, committed);
            if lines.len() + 1 >= MAX_TEXT_LINES {
                lines.push(&value[line_start..]);
                return lines;
            }
        }

        // Whatever is left before this boundary now has no break inside it. If
        // it still does not fit, it is one word wider than the column, and it
        // is broken between characters until the remainder fits — repeatedly,
        // because a word can be several lines wide on its own.
        while line_start < boundary
            && !fits(&value[line_start..boundary], font_size, max_width, measurer)
        {
            let split =
                split_long_word(&value[line_start..boundary], font_size, max_width, measurer);
            lines.push(&value[line_start..line_start + split]);
            line_start += split;
            if lines.len() + 1 >= MAX_TEXT_LINES {
                lines.push(&value[line_start..]);
                return lines;
            }
        }
        committed = boundary;
    }

    if line_start < value.len() {
        lines.push(&value[line_start..]);
    }
    if lines.is_empty() {
        lines.push(value);
    }
    lines
}

/// Returns the height of `line_count` stacked lines at one measured line height.
///
/// Every line of one text value is spaced by the same amount, taken from the
/// value as a whole. Spacing each line by its own measured height would make a
/// paragraph's line rhythm depend on which characters landed on which line.
#[must_use]
pub fn wrapped_height(line_count: usize, single_line_height: f32) -> f32 {
    single_line_height * line_count.max(1) as f32
}

fn fits(candidate: &str, font_size: u16, max_width: f32, measurer: &dyn TextMeasurer) -> bool {
    measurer.measure(candidate, font_size).sanitized().width <= max_width
}

/// Yields every byte index at which a line may break, in ascending order.
///
/// A break is offered *after* each space run and at the end of the value, so a
/// candidate slice always ends on a whole word.
fn break_candidates(value: &str) -> impl Iterator<Item = usize> + '_ {
    value
        .char_indices()
        .filter(|(index, character)| {
            *character == ' ' && !value[..*index].is_empty() && !value[*index..].trim().is_empty()
        })
        .map(|(index, _)| index)
        .chain(std::iter::once(value.len()))
}

fn skip_space(value: &str, from: usize) -> usize {
    let remainder = &value[from..];
    from + remainder.len() - remainder.trim_start_matches(' ').len()
}

/// Returns the byte length of the longest prefix of `word` that fits.
///
/// Never returns zero: a column too narrow for even one character would
/// otherwise produce a line holding nothing and an unmoving cursor.
fn split_long_word(
    word: &str,
    font_size: u16,
    max_width: f32,
    measurer: &dyn TextMeasurer,
) -> usize {
    let mut committed = 0;
    for (index, character) in word.char_indices() {
        let end = index + character.len_utf8();
        if fits(&word[..end], font_size, max_width, measurer) {
            committed = end;
        } else {
            break;
        }
    }
    if committed == 0 {
        word.chars()
            .next()
            .map_or(word.len(), |character| character.len_utf8())
    } else {
        committed
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_TEXT_LINES, wrap_text, wrapped_height};
    use crate::{TextMeasurer, UiSize};

    /// Ten logical pixels per character, so a width is a character count.
    struct FixedMeasurer;

    impl TextMeasurer for FixedMeasurer {
        fn measure(&self, text: &str, font_size: u16) -> UiSize {
            UiSize::new(text.chars().count() as f32 * 10.0, f32::from(font_size))
        }
    }

    fn wrap(value: &str, columns: usize) -> Vec<&str> {
        wrap_text(value, 10, columns as f32 * 10.0, &FixedMeasurer)
    }

    #[test]
    fn text_that_already_fits_stays_one_line() {
        assert_eq!(wrap("short enough", 40), ["short enough"]);
    }

    #[test]
    fn an_empty_value_still_occupies_one_line() {
        // Height comes from the line count, so returning nothing here would
        // collapse an empty text node and shift every sibling above it.
        assert_eq!(wrap("", 40), [""]);
    }

    #[test]
    fn breaking_happens_at_spaces_and_drops_the_space() {
        // "alpha beta" is 10 characters, so a 7-column width admits "alpha"
        // only. The space is consumed by the break rather than starting the
        // next line with it.
        assert_eq!(wrap("alpha beta", 7), ["alpha", "beta"]);
    }

    #[test]
    fn every_line_fits_within_the_width() {
        let value = "the quick brown fox jumps over the lazy dog near the river bank";
        for columns in [8, 11, 17, 23, 40] {
            for line in wrap(value, columns) {
                assert!(
                    line.chars().count() <= columns,
                    "line {line:?} exceeds {columns} columns"
                );
            }
        }
    }

    #[test]
    fn wrapping_preserves_every_word_in_order() {
        let value = "the quick brown fox jumps over the lazy dog";
        let wrapped = wrap(value, 12).join(" ");
        assert_eq!(wrapped, value);
    }

    #[test]
    fn a_word_wider_than_the_column_is_broken_between_characters() {
        // The alternative is the clipping this exists to remove: a long word
        // left whole would run past the edge and take the rest with it.
        assert_eq!(
            wrap("supercalifragilistic", 6),
            ["superc", "alifra", "gilist", "ic"]
        );
    }

    #[test]
    fn a_long_word_does_not_swallow_the_words_after_it() {
        let wrapped = wrap("enormouslylong tail", 6);
        assert_eq!(wrapped.last(), Some(&"tail"));
    }

    #[test]
    fn a_column_narrower_than_one_character_still_advances() {
        // Zero characters per line would loop forever rather than render badly.
        let wrapped = wrap("abc", 0);
        assert!(!wrapped.is_empty());
        assert_eq!(wrapped.concat(), "abc");
    }

    #[test]
    fn an_unusable_width_returns_the_value_unwrapped() {
        assert_eq!(wrap_text("value", 10, f32::NAN, &FixedMeasurer), ["value"]);
        assert_eq!(wrap_text("value", 10, -5.0, &FixedMeasurer), ["value"]);
    }

    #[test]
    fn wrapping_is_bounded_and_keeps_the_remainder_on_the_last_line() {
        let value = "word ".repeat(400);
        let wrapped = wrap(value.trim_end(), 4);
        assert!(
            wrapped.len() <= MAX_TEXT_LINES,
            "{} lines exceeded the ceiling",
            wrapped.len()
        );
        // Bounded, not truncated. What did not fit is still present on the
        // final line, where the client rectangle clips it exactly as an
        // unwrapped value was clipped before.
        assert!(wrapped.last().expect("a final line").ends_with("word"));
    }

    #[test]
    fn wrapping_again_at_its_own_widest_line_reproduces_the_same_lines() {
        // Layout reports a wrapped run's width as its widest line, and the host
        // draws by wrapping at the bounds it is given. If those two disagreed,
        // the measured height and the painted lines would drift apart. Greedy
        // breaking makes them agree: a word rejected at the wider width is
        // still rejected at the narrower one.
        let value = "the quick brown fox jumps over the lazy dog near the river";
        for columns in [7, 13, 19, 26] {
            let first = wrap(value, columns);
            let widest = first
                .iter()
                .map(|line| line.chars().count())
                .max()
                .expect("at least one line");
            assert_eq!(first, wrap(value, widest), "unstable at {columns} columns");
        }
    }

    #[test]
    fn multibyte_text_breaks_only_on_character_boundaries() {
        // Slicing mid-character would panic rather than render badly, so this
        // is a crash test as much as a layout one.
        let value = "ααααα βββββ γγγγγ";
        let wrapped = wrap(value, 6);
        assert_eq!(wrapped.join(" "), value);
        let broken = wrap("ααααααααααα", 4);
        assert_eq!(broken.concat(), "ααααααααααα");
    }

    #[test]
    fn stacked_line_height_multiplies_one_measured_line() {
        assert_eq!(wrapped_height(3, 20.0), 60.0);
        // A zero line count still reserves one line, matching wrap_text never
        // returning an empty result.
        assert_eq!(wrapped_height(0, 20.0), 20.0);
    }
}
