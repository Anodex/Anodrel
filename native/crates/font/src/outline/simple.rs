//! Packed simple-glyph contour parsing.

use crate::{
    bytes::Bytes,
    outline::{
        GlyphBounds, GlyphOutline, GlyphOutlineError, GlyphPoint,
        types::{MAX_CONTOURS, MAX_POINTS},
    },
};

const HEADER_LENGTH: usize = 10;
const ON_CURVE: u8 = 0x01;
const X_SHORT: u8 = 0x02;
const Y_SHORT: u8 = 0x04;
const REPEAT: u8 = 0x08;
const X_SAME_OR_POSITIVE: u8 = 0x10;
const Y_SAME_OR_POSITIVE: u8 = 0x20;
const OVERLAP_SIMPLE: u8 = 0x40;
const RESERVED: u8 = 0x80;

/// Parses one located simple glyph into an owned bounded outline.
pub(super) fn parse(glyph: Bytes<'_>) -> Result<GlyphOutline, GlyphOutlineError> {
    let contour_count = glyph.i16(0).ok_or(GlyphOutlineError::MalformedOutline)?;
    let bounds = GlyphBounds::new(
        glyph.i16(2).ok_or(GlyphOutlineError::MalformedOutline)?,
        glyph.i16(4).ok_or(GlyphOutlineError::MalformedOutline)?,
        glyph.i16(6).ok_or(GlyphOutlineError::MalformedOutline)?,
        glyph.i16(8).ok_or(GlyphOutlineError::MalformedOutline)?,
    )
    .ok_or(GlyphOutlineError::MalformedOutline)?;
    if contour_count == 0 {
        return parse_empty(glyph, bounds);
    }
    if contour_count < 0 {
        return Err(GlyphOutlineError::MalformedOutline);
    }
    parse_simple(
        glyph,
        usize::try_from(contour_count).map_err(|_| GlyphOutlineError::MalformedOutline)?,
        bounds,
    )
}

fn parse_empty(glyph: Bytes<'_>, bounds: GlyphBounds) -> Result<GlyphOutline, GlyphOutlineError> {
    if glyph.len() == HEADER_LENGTH {
        return Ok(GlyphOutline::new(bounds, Vec::new(), Vec::new()));
    }
    let mut cursor = HEADER_LENGTH;
    let instruction_length = usize::from(read_u16(glyph, &mut cursor)?);
    skip(glyph, &mut cursor, instruction_length)?;
    if !glyph.zero_padding_from(cursor) {
        return Err(GlyphOutlineError::MalformedOutline);
    }
    Ok(GlyphOutline::new(bounds, Vec::new(), Vec::new()))
}

fn parse_simple(
    glyph: Bytes<'_>,
    contour_count: usize,
    bounds: GlyphBounds,
) -> Result<GlyphOutline, GlyphOutlineError> {
    if contour_count > MAX_CONTOURS {
        return Err(GlyphOutlineError::ComplexityLimitExceeded);
    }
    let mut cursor = HEADER_LENGTH;
    let contour_ends = read_contour_ends(glyph, &mut cursor, contour_count)?;
    let point_count = contour_ends.last().map_or(0, |end| end + 1);
    if point_count > MAX_POINTS {
        return Err(GlyphOutlineError::ComplexityLimitExceeded);
    }
    let instruction_length = usize::from(read_u16(glyph, &mut cursor)?);
    skip(glyph, &mut cursor, instruction_length)?;
    let flags = read_flags(glyph, &mut cursor, point_count)?;
    let mut points = read_x_coordinates(glyph, &mut cursor, &flags)?;
    read_y_coordinates(glyph, &mut cursor, &flags, &mut points)?;
    if !glyph.zero_padding_from(cursor) {
        return Err(GlyphOutlineError::MalformedOutline);
    }
    Ok(GlyphOutline::new(bounds, points, contour_ends))
}

fn read_contour_ends(
    glyph: Bytes<'_>,
    cursor: &mut usize,
    contour_count: usize,
) -> Result<Vec<usize>, GlyphOutlineError> {
    let mut ends = Vec::with_capacity(contour_count);
    for _ in 0..contour_count {
        let end = usize::from(read_u16(glyph, cursor)?);
        if ends.last().is_some_and(|previous| end <= *previous) {
            return Err(GlyphOutlineError::MalformedOutline);
        }
        ends.push(end);
    }
    Ok(ends)
}

fn read_flags(
    glyph: Bytes<'_>,
    cursor: &mut usize,
    point_count: usize,
) -> Result<Vec<u8>, GlyphOutlineError> {
    let mut flags = Vec::with_capacity(point_count);
    while flags.len() < point_count {
        let flag = read_u8(glyph, cursor)?;
        if flag & RESERVED != 0 {
            return Err(GlyphOutlineError::MalformedOutline);
        }
        let repeat_count = if flag & REPEAT == 0 {
            1
        } else {
            usize::from(read_u8(glyph, cursor)?) + 1
        };
        if repeat_count > point_count - flags.len()
            || flag & OVERLAP_SIMPLE != 0 && (!flags.is_empty() || repeat_count != 1)
        {
            return Err(GlyphOutlineError::MalformedOutline);
        }
        flags.extend(std::iter::repeat_n(flag & !REPEAT, repeat_count));
    }
    Ok(flags)
}

fn read_x_coordinates(
    glyph: Bytes<'_>,
    cursor: &mut usize,
    flags: &[u8],
) -> Result<Vec<GlyphPoint>, GlyphOutlineError> {
    let mut coordinate = 0_i32;
    let mut points = Vec::with_capacity(flags.len());
    for &flag in flags {
        coordinate = coordinate
            .checked_add(i32::from(read_delta(
                glyph,
                cursor,
                flag,
                X_SHORT,
                X_SAME_OR_POSITIVE,
            )?))
            .ok_or(GlyphOutlineError::MalformedOutline)?;
        let x = i16::try_from(coordinate).map_err(|_| GlyphOutlineError::MalformedOutline)?;
        points.push(GlyphPoint::new(x, 0, flag & ON_CURVE != 0));
    }
    Ok(points)
}

fn read_y_coordinates(
    glyph: Bytes<'_>,
    cursor: &mut usize,
    flags: &[u8],
    points: &mut [GlyphPoint],
) -> Result<(), GlyphOutlineError> {
    let mut coordinate = 0_i32;
    for (point, &flag) in points.iter_mut().zip(flags) {
        coordinate = coordinate
            .checked_add(i32::from(read_delta(
                glyph,
                cursor,
                flag,
                Y_SHORT,
                Y_SAME_OR_POSITIVE,
            )?))
            .ok_or(GlyphOutlineError::MalformedOutline)?;
        point.set_y(i16::try_from(coordinate).map_err(|_| GlyphOutlineError::MalformedOutline)?);
    }
    Ok(())
}

fn read_delta(
    glyph: Bytes<'_>,
    cursor: &mut usize,
    flag: u8,
    short: u8,
    same_or_positive: u8,
) -> Result<i16, GlyphOutlineError> {
    if flag & short != 0 {
        let magnitude = i16::from(read_u8(glyph, cursor)?);
        return Ok(if flag & same_or_positive != 0 {
            magnitude
        } else {
            -magnitude
        });
    }
    if flag & same_or_positive != 0 {
        return Ok(0);
    }
    read_i16(glyph, cursor)
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
