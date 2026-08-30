//! Closed parsing for compositor events used by the fixed Linux Lab.

use super::{
    error::LinuxWaylandError,
    globals::Globals,
    raw::Connection,
    wire::{Message, WireError},
};

const DISPLAY: u32 = 1;
const XRGB8888: u32 = 1;

pub(super) fn next_message(
    connection: &Connection,
    input: &mut Vec<u8>,
) -> Result<Message, LinuxWaylandError> {
    loop {
        if input.len() >= 8 {
            let word = u32::from_ne_bytes(
                input[4..8]
                    .try_into()
                    .map_err(|_| LinuxWaylandError::ProtocolRejected)?,
            );
            let size = (word >> 16) as usize;
            if !(8..=65_532).contains(&size) || !size.is_multiple_of(4) {
                return Err(LinuxWaylandError::ProtocolRejected);
            }
            if input.len() >= size {
                let frame: Vec<_> = input.drain(..size).collect();
                return Message::from_frame(&frame).map_err(protocol_error);
            }
        }
        if input.len() >= 65_532 {
            return Err(LinuxWaylandError::ProtocolRejected);
        }
        connection
            .receive(input)
            .map_err(|_| LinuxWaylandError::DesktopUnavailable)?;
    }
}

pub(super) fn display_event(message: &Message) -> Result<(), LinuxWaylandError> {
    if message.object != DISPLAY {
        return Err(LinuxWaylandError::ProtocolRejected);
    }
    let mut reader = message.reader();
    match message.opcode {
        0 => {
            reader.uint().map_err(protocol_error)?;
            reader.uint().map_err(protocol_error)?;
            reader.string().map_err(protocol_error)?;
            reader.finish().map_err(protocol_error)?;
            Err(LinuxWaylandError::ProtocolRejected)
        }
        1 => {
            reader.uint().map_err(protocol_error)?;
            reader.finish().map_err(protocol_error)
        }
        _ => Err(LinuxWaylandError::ProtocolRejected),
    }
}

pub(super) fn registry_event(
    message: &Message,
    globals: Option<&mut Globals>,
) -> Result<(), LinuxWaylandError> {
    let mut reader = message.reader();
    match message.opcode {
        0 => {
            let name = reader.uint().map_err(protocol_error)?;
            let interface = reader.string().map_err(protocol_error)?;
            let version = reader.uint().map_err(protocol_error)?;
            reader.finish().map_err(protocol_error)?;
            if let Some(globals) = globals {
                globals.record(name, interface, version);
            }
            Ok(())
        }
        1 => {
            reader.uint().map_err(protocol_error)?;
            reader.finish().map_err(protocol_error)
        }
        _ => Err(LinuxWaylandError::ProtocolRejected),
    }
}

pub(super) fn shm_event(
    message: &Message,
    xrgb: Option<&mut bool>,
) -> Result<(), LinuxWaylandError> {
    if message.opcode != 0 {
        return Err(LinuxWaylandError::ProtocolRejected);
    }
    let mut reader = message.reader();
    let format = reader.uint().map_err(protocol_error)?;
    reader.finish().map_err(protocol_error)?;
    if let Some(xrgb) = xrgb {
        *xrgb |= format == XRGB8888;
    }
    Ok(())
}

pub(super) fn seat_has_pointer(message: &Message) -> Result<bool, LinuxWaylandError> {
    if message.opcode != 0 {
        return Err(LinuxWaylandError::ProtocolRejected);
    }
    let mut reader = message.reader();
    let capabilities = reader.uint().map_err(protocol_error)?;
    reader.finish().map_err(protocol_error)?;
    Ok(capabilities & 1 != 0)
}

pub(super) fn toplevel_closed(message: &Message) -> Result<bool, LinuxWaylandError> {
    let mut reader = message.reader();
    match message.opcode {
        0 => {
            reader.int().map_err(protocol_error)?;
            reader.int().map_err(protocol_error)?;
            reader.array().map_err(protocol_error)?;
            reader.finish().map_err(protocol_error)?;
            Ok(false)
        }
        1 => {
            reader.finish().map_err(protocol_error)?;
            Ok(true)
        }
        _ => Err(LinuxWaylandError::ProtocolRejected),
    }
}

pub(super) fn protocol_error(_: WireError) -> LinuxWaylandError {
    LinuxWaylandError::ProtocolRejected
}
