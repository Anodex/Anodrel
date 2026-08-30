//! Private pointer-state and fixed activation semantics for the Linux Lab.

use super::{
    error::LinuxWaylandError,
    wire::{Message, Reader},
};

const FIXED_SCALE: i32 = 256;
const TARGET_LEFT: i32 = 330 * FIXED_SCALE;
const TARGET_TOP: i32 = 416 * FIXED_SCALE;
const TARGET_RIGHT: i32 = 630 * FIXED_SCALE;
const TARGET_BOTTOM: i32 = 512 * FIXED_SCALE;
const LEFT_BUTTON: u32 = 272;
const RELEASED: u32 = 0;
const PRESSED: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PointerResult {
    None,
    Activated,
}

#[derive(Default)]
pub(super) struct PointerState {
    focused: bool,
    x: i32,
    y: i32,
    left_armed: bool,
}

impl PointerState {
    pub(super) fn dispatch(
        &mut self,
        message: &Message,
        surface: u32,
    ) -> Result<PointerResult, LinuxWaylandError> {
        match message.opcode {
            0 => self.enter(message.reader(), surface),
            1 => self.leave(message.reader(), surface),
            2 => self.motion(message.reader()),
            3 => self.button(message.reader()),
            4 => self.axis(message.reader()),
            _ => Err(LinuxWaylandError::ProtocolRejected),
        }
    }

    fn enter(
        &mut self,
        mut reader: Reader<'_>,
        surface: u32,
    ) -> Result<PointerResult, LinuxWaylandError> {
        reader.uint().map_err(reject)?;
        let entered_surface = reader.uint().map_err(reject)?;
        let x = reader.int().map_err(reject)?;
        let y = reader.int().map_err(reject)?;
        reader.finish().map_err(reject)?;
        if entered_surface != surface {
            return Err(LinuxWaylandError::ProtocolRejected);
        }
        self.focused = true;
        self.x = x;
        self.y = y;
        self.left_armed = false;
        Ok(PointerResult::None)
    }

    fn leave(
        &mut self,
        mut reader: Reader<'_>,
        surface: u32,
    ) -> Result<PointerResult, LinuxWaylandError> {
        reader.uint().map_err(reject)?;
        let left_surface = reader.uint().map_err(reject)?;
        reader.finish().map_err(reject)?;
        if !self.focused || left_surface != surface {
            return Err(LinuxWaylandError::ProtocolRejected);
        }
        self.focused = false;
        self.left_armed = false;
        Ok(PointerResult::None)
    }

    fn motion(&mut self, mut reader: Reader<'_>) -> Result<PointerResult, LinuxWaylandError> {
        reader.uint().map_err(reject)?;
        let x = reader.int().map_err(reject)?;
        let y = reader.int().map_err(reject)?;
        reader.finish().map_err(reject)?;
        if !self.focused {
            return Err(LinuxWaylandError::ProtocolRejected);
        }
        self.x = x;
        self.y = y;
        Ok(PointerResult::None)
    }

    fn button(&mut self, mut reader: Reader<'_>) -> Result<PointerResult, LinuxWaylandError> {
        reader.uint().map_err(reject)?;
        reader.uint().map_err(reject)?;
        let button = reader.uint().map_err(reject)?;
        let state = reader.uint().map_err(reject)?;
        reader.finish().map_err(reject)?;
        if !self.focused || !matches!(state, RELEASED | PRESSED) {
            return Err(LinuxWaylandError::ProtocolRejected);
        }
        if button != LEFT_BUTTON {
            return Ok(PointerResult::None);
        }
        if state == PRESSED {
            self.left_armed = self.in_target();
            return Ok(PointerResult::None);
        }
        let activated = self.left_armed && self.in_target();
        self.left_armed = false;
        Ok(if activated {
            PointerResult::Activated
        } else {
            PointerResult::None
        })
    }

    fn axis(&mut self, mut reader: Reader<'_>) -> Result<PointerResult, LinuxWaylandError> {
        reader.uint().map_err(reject)?;
        reader.uint().map_err(reject)?;
        reader.int().map_err(reject)?;
        reader.finish().map_err(reject)?;
        self.focused
            .then_some(PointerResult::None)
            .ok_or(LinuxWaylandError::ProtocolRejected)
    }

    fn in_target(&self) -> bool {
        (TARGET_LEFT..TARGET_RIGHT).contains(&self.x)
            && (TARGET_TOP..TARGET_BOTTOM).contains(&self.y)
    }
}

fn reject(_: super::wire::WireError) -> LinuxWaylandError {
    LinuxWaylandError::ProtocolRejected
}

#[cfg(test)]
mod tests {
    use super::{PointerResult, PointerState};
    use crate::linux::wire::Message;

    const POINTER: u32 = 9;
    const SURFACE: u32 = 7;

    #[test]
    fn left_click_inside_the_fixed_target_activates_once() {
        let mut state = PointerState::default();
        assert_eq!(
            state.dispatch(&event(0, &[1, SURFACE, 400 * 256, 450 * 256]), SURFACE),
            Ok(PointerResult::None)
        );
        assert_eq!(
            state.dispatch(&event(3, &[2, 3, 272, 1]), SURFACE),
            Ok(PointerResult::None)
        );
        assert_eq!(
            state.dispatch(&event(3, &[2, 4, 272, 0]), SURFACE),
            Ok(PointerResult::Activated)
        );
        assert_eq!(
            state.dispatch(&event(3, &[2, 5, 272, 0]), SURFACE),
            Ok(PointerResult::None)
        );
    }

    #[test]
    fn press_and_release_must_stay_inside_the_fixed_target() {
        let mut state = PointerState::default();
        state
            .dispatch(&event(0, &[1, SURFACE, 400 * 256, 450 * 256]), SURFACE)
            .expect("enter is valid");
        state
            .dispatch(&event(3, &[2, 3, 272, 1]), SURFACE)
            .expect("press is valid");
        state
            .dispatch(&event(2, &[4, 640 * 256, 450 * 256]), SURFACE)
            .expect("motion is valid");
        assert_eq!(
            state.dispatch(&event(3, &[2, 5, 272, 0]), SURFACE),
            Ok(PointerResult::None)
        );
    }

    #[test]
    fn pointer_events_cannot_target_another_surface() {
        let mut state = PointerState::default();
        assert!(
            state
                .dispatch(&event(0, &[1, SURFACE + 1, 400 * 256, 450 * 256]), SURFACE)
                .is_err()
        );
    }

    fn event(opcode: u16, words: &[u32]) -> Message {
        let size = 8 + words.len() * 4;
        let mut frame = Vec::with_capacity(size);
        frame.extend(POINTER.to_ne_bytes());
        frame.extend(((size as u32) << 16 | u32::from(opcode)).to_ne_bytes());
        for word in words {
            frame.extend(word.to_ne_bytes());
        }
        Message::from_frame(&frame).expect("test event is well formed")
    }
}
