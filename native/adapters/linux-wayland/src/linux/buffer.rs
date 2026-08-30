//! Fixed double-buffer availability and canvas copying.

use anodrel_canvas::Canvas;

use super::raw::SharedMemory;

pub(super) const BUFFER_COUNT: usize = 2;

pub(super) struct Buffer {
    pub(super) object: u32,
    available: bool,
    memory: SharedMemory,
}

impl Buffer {
    pub(super) fn new(object: u32, memory: SharedMemory) -> Self {
        Self {
            object,
            available: true,
            memory,
        }
    }

    pub(super) fn is_available(&self) -> bool {
        self.available
    }

    pub(super) fn occupy(&mut self, canvas: &Canvas) {
        let source = unsafe {
            std::slice::from_raw_parts(
                canvas.pixels().as_ptr().cast::<u8>(),
                canvas.pixels().len() * size_of::<u32>(),
            )
        };
        self.memory.bytes_mut().copy_from_slice(source);
        self.available = false;
    }

    pub(super) fn release(&mut self) {
        self.available = true;
    }
}

const fn size_of<T>() -> usize {
    std::mem::size_of::<T>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_two_buffers_are_available_at_once() {
        let mut available = [true; super::BUFFER_COUNT];
        available.fill(false);
        assert!(!available.iter().any(|slot| *slot));
        available[1] = true;
        assert_eq!(available.iter().filter(|slot| **slot).count(), 1);
    }
}
