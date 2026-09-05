//! Bounded parsing for translation-only TrueType composite records.

use crate::{
    GlyphId,
    bytes::Bytes,
    outline::{GlyphBounds, GlyphOutlineError},
};

const HEADER_LENGTH: usize = 10;
pub(super) const MAX_COMPONENTS: usize = 128;
const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const ARGS_ARE_XY_VALUES: u16 = 0x0002;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
const WE_HAVE_INSTRUCTIONS: u16 = 0x0100;
const OVERLAP_COMPOUND: u16 = 0x0400;
const SCALED_COMPONENT_OFFSET: u16 = 0x0800;
const UNSCALED_COMPONENT_OFFSET: u16 = 0x1000;
const RESERVED_FLAGS: u16 = 0xE010;
const TRANSFORM_FLAGS: u16 = WE_HAVE_A_SCALE
    | WE_HAVE_AN_X_AND_Y_SCALE
    | WE_HAVE_A_TWO_BY_TWO
    | SCALED_COMPONENT_OFFSET
    | UNSCALED_COMPONENT_OFFSET;

pub(super) struct CompositeGlyph {
    pub(super) bounds: GlyphBounds,
    pub(super) components: Vec<Component>,
}

pub(super) struct Component {
    pub(super) glyph: GlyphId,
    pub(super) x_offset: i16,
    pub(super) y_offset: i16,
}

/// Parses only whole-design-unit x/y component translation.
pub(super) fn parse(glyph: Bytes<'_>) -> Result<CompositeGlyph, GlyphOutlineError> {
    if glyph.i16(0) != Some(-1) {
        return Err(GlyphOutlineError::MalformedOutline);
    }
    let bounds = GlyphBounds::new(
        glyph.i16(2).ok_or(GlyphOutlineError::MalformedOutline)?,
        glyph.i16(4).ok_or(GlyphOutlineError::MalformedOutline)?,
        glyph.i16(6).ok_or(GlyphOutlineError::MalformedOutline)?,
        glyph.i16(8).ok_or(GlyphOutlineError::MalformedOutline)?,
    )
    .ok_or(GlyphOutlineError::MalformedOutline)?;
    let mut cursor = HEADER_LENGTH;
    let mut components = Vec::new();
    let mut has_instructions = false;
    loop {
        if components.len() == MAX_COMPONENTS {
            return Err(GlyphOutlineError::ComplexityLimitExceeded);
        }
        let flags = read_u16(glyph, &mut cursor)?;
        if flags & RESERVED_FLAGS != 0 || flags & OVERLAP_COMPOUND != 0 && !components.is_empty() {
            return Err(GlyphOutlineError::MalformedOutline);
        }
        let glyph_id = GlyphId::new(read_u16(glyph, &mut cursor)?);
        let (first, second) = read_arguments(glyph, &mut cursor, flags)?;
        if flags & ARGS_ARE_XY_VALUES == 0 {
            return if components.is_empty() {
                Err(GlyphOutlineError::MalformedOutline)
            } else {
                Err(GlyphOutlineError::CompositePointAttachmentUnsupported)
            };
        }
        if flags & TRANSFORM_FLAGS != 0 {
            return Err(GlyphOutlineError::CompositeTransformUnsupported);
        }
        components.push(Component {
            glyph: glyph_id,
            x_offset: first,
            y_offset: second,
        });
        has_instructions |= flags & WE_HAVE_INSTRUCTIONS != 0;
        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }
    if has_instructions {
        let instruction_length = usize::from(read_u16(glyph, &mut cursor)?);
        skip(glyph, &mut cursor, instruction_length)?;
    }
    if !glyph.zero_padding_from(cursor) {
        return Err(GlyphOutlineError::MalformedOutline);
    }
    Ok(CompositeGlyph { bounds, components })
}

fn read_arguments(
    glyph: Bytes<'_>,
    cursor: &mut usize,
    flags: u16,
) -> Result<(i16, i16), GlyphOutlineError> {
    if flags & ARG_1_AND_2_ARE_WORDS != 0 {
        return Ok((read_i16(glyph, cursor)?, read_i16(glyph, cursor)?));
    }
    Ok((
        i16::from(read_u8(glyph, cursor)? as i8),
        i16::from(read_u8(glyph, cursor)? as i8),
    ))
}

fn read_u8(glyph: Bytes<'_>, cursor: &mut usize) -> Result<u8, GlyphOutlineError> {
    let value = glyph
        .u8(*cursor)
        .ok_or(GlyphOutlineError::MalformedOutline)?;
    *cursor = cursor
        .checked_add(1)
        .ok_or(GlyphOutlineError::MalformedOutline)?;
    Ok(value)
}

fn read_u16(glyph: Bytes<'_>, cursor: &mut usize) -> Result<u16, GlyphOutlineError> {
    let value = glyph
        .u16(*cursor)
        .ok_or(GlyphOutlineError::MalformedOutline)?;
    *cursor = cursor
        .checked_add(2)
        .ok_or(GlyphOutlineError::MalformedOutline)?;
    Ok(value)
}

fn read_i16(glyph: Bytes<'_>, cursor: &mut usize) -> Result<i16, GlyphOutlineError> {
    let value = glyph
        .i16(*cursor)
        .ok_or(GlyphOutlineError::MalformedOutline)?;
    *cursor = cursor
        .checked_add(2)
        .ok_or(GlyphOutlineError::MalformedOutline)?;
    Ok(value)
}

fn skip(glyph: Bytes<'_>, cursor: &mut usize, length: usize) -> Result<(), GlyphOutlineError> {
    glyph
        .range(*cursor, length)
        .ok_or(GlyphOutlineError::MalformedOutline)?;
    *cursor = cursor
        .checked_add(length)
        .ok_or(GlyphOutlineError::MalformedOutline)?;
    Ok(())
}
