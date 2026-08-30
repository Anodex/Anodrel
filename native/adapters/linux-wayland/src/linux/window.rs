//! Fixed Wayland diagnostic-window lifecycle.

use std::{
    fmt,
    time::{Duration, Instant},
};

use anodrel_canvas::Canvas;
use anodrel_linux_lab_surface::{LAB_HEIGHT, LAB_WIDTH};

use super::{
    buffer::{BUFFER_COUNT, Buffer},
    error::LinuxWaylandError,
    events::{
        display_event, next_message, next_message_with_timeout, protocol_error, registry_event,
        seat_has_pointer, shm_event, toplevel_closed,
    },
    globals::{Global, Globals},
    locator::Locator,
    pointer::{PointerResult, PointerState},
    raw::{Connection, SharedMemory},
    wire::{Message, ObjectIds, Request},
};

const DISPLAY: u32 = 1;
const REGISTRY: u32 = 2;
const XRGB8888: u32 = 1;
const COMPOSITOR_VERSION: u32 = 1;
const SEAT_VERSION: u32 = 1;

/// Closed diagnostic outcome from the Linux Lab event loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxWaylandLabEvent {
    /// The fixed local pointer activation completed.
    Activated,
    /// The compositor requested the fixed diagnostic window close.
    Closed,
}

/// One fixed direct-Wayland development diagnostic with no window API surface.
pub struct LinuxWaylandLab {
    connection: Connection,
    input: Vec<u8>,
    objects: Objects,
    buffers: [Buffer; BUFFER_COUNT],
    pointer: Option<PointerState>,
}

impl LinuxWaylandLab {
    /// Opens the fixed Anodrel Linux Lab after one complete compositor handshake.
    pub fn open() -> Result<Self, LinuxWaylandError> {
        let locator =
            Locator::from_environment().map_err(|_| LinuxWaylandError::DesktopUnavailable)?;
        let connection = Connection::connect(locator.path())
            .map_err(|_| LinuxWaylandError::DesktopUnavailable)?;
        let mut setup = Setup {
            connection,
            input: Vec::new(),
            ids: ObjectIds::new(),
        };

        let registry = setup.allocate()?;
        debug_assert_eq!(registry, REGISTRY);
        setup.send(new_id_request(DISPLAY, 1, registry))?;
        let registry_sync = setup.allocate()?;
        setup.send(new_id_request(DISPLAY, 0, registry_sync))?;
        let globals = setup.wait_for_globals(registry, registry_sync)?;

        let compositor = setup.allocate()?;
        setup.bind(
            globals
                .compositor()
                .ok_or(LinuxWaylandError::RequiredSupportUnavailable)?,
            "wl_compositor",
            COMPOSITOR_VERSION,
            compositor,
        )?;
        let shm = setup.allocate()?;
        setup.bind(
            globals
                .shm()
                .ok_or(LinuxWaylandError::RequiredSupportUnavailable)?,
            "wl_shm",
            1,
            shm,
        )?;
        let xdg_wm_base = setup.allocate()?;
        setup.bind(
            globals
                .xdg_wm_base()
                .ok_or(LinuxWaylandError::RequiredSupportUnavailable)?,
            "xdg_wm_base",
            1,
            xdg_wm_base,
        )?;

        let seat = if let Some(global) = globals.seat() {
            let seat = setup.allocate()?;
            setup.bind(global, "wl_seat", SEAT_VERSION, seat)?;
            Some(seat)
        } else {
            None
        };

        let format_sync = setup.allocate()?;
        setup.send(new_id_request(DISPLAY, 0, format_sync))?;
        let pointer_supported =
            setup.wait_for_formats_and_pointer(registry, shm, seat, format_sync)?;
        let pointer = if pointer_supported {
            let seat = seat.ok_or(LinuxWaylandError::ProtocolRejected)?;
            let pointer = setup.allocate()?;
            setup.send(new_id_request(seat, 0, pointer))?;
            Some(pointer)
        } else {
            None
        };

        let surface = setup.allocate()?;
        setup.send(new_id_request(compositor, 0, surface))?;
        let xdg_surface = setup.allocate()?;
        let mut get_xdg_surface = Request::new(xdg_wm_base, 2);
        get_xdg_surface.uint(xdg_surface);
        get_xdg_surface.uint(surface);
        setup.send(get_xdg_surface)?;
        let toplevel = setup.allocate()?;
        setup.send(new_id_request(xdg_surface, 1, toplevel))?;
        setup.set_toplevel_metadata(toplevel)?;
        setup.send(Request::new(surface, 6))?;

        let objects = Objects {
            registry,
            shm,
            xdg_wm_base,
            surface,
            xdg_surface,
            toplevel,
            seat,
            pointer,
        };
        setup.wait_for_initial_configure(&objects)?;
        let buffers = setup.create_buffers(shm)?;
        Ok(Self {
            connection: setup.connection,
            input: setup.input,
            objects,
            buffers,
            pointer: pointer.map(|_| PointerState::default()),
        })
    }

    /// Copies one complete fixed-size canvas into an available compositor buffer.
    pub fn present(&mut self, canvas: &Canvas) -> Result<(), LinuxWaylandError> {
        if canvas.width() != LAB_WIDTH || canvas.height() != LAB_HEIGHT {
            return Err(LinuxWaylandError::CanvasSizeMismatch);
        }
        let buffer = self
            .buffers
            .iter_mut()
            .find(|buffer| buffer.is_available())
            .ok_or(LinuxWaylandError::Backpressured)?;
        buffer.occupy(canvas);

        let mut attach = Request::new(self.objects.surface, 1);
        attach.uint(buffer.object);
        attach.int(0);
        attach.int(0);
        self.send(attach)?;
        let mut damage = Request::new(self.objects.surface, 2);
        damage.int(0);
        damage.int(0);
        damage.int(LAB_WIDTH as i32);
        damage.int(LAB_HEIGHT as i32);
        self.send(damage)?;
        self.send(Request::new(self.objects.surface, 6))
    }

    /// Blocks only in the diagnostic's own event loop until the desktop closes it.
    pub fn wait_for_close(&mut self) -> Result<(), LinuxWaylandError> {
        loop {
            if self.wait_for_lab_event()? == LinuxWaylandLabEvent::Closed {
                return Ok(());
            }
        }
    }

    /// Waits for one closed local diagnostic outcome from the compositor loop.
    pub fn wait_for_lab_event(&mut self) -> Result<LinuxWaylandLabEvent, LinuxWaylandError> {
        loop {
            let message = self.next_message()?;
            if let Some(event) = self.dispatch(message)? {
                return Ok(event);
            }
        }
    }

    /// Waits no longer than `timeout` for one closed local diagnostic outcome.
    ///
    /// This stays inside the fixed diagnostic host boundary. It exposes no
    /// Wayland descriptor, native wait handle, callback, or application event.
    pub fn wait_for_lab_event_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<LinuxWaylandLabEvent>, LinuxWaylandError> {
        let started = Instant::now();
        loop {
            let remaining = timeout.saturating_sub(started.elapsed());
            let Some(message) = self.next_message_with_timeout(remaining)? else {
                return Ok(None);
            };
            if let Some(event) = self.dispatch(message)? {
                return Ok(Some(event));
            }
        }
    }

    fn dispatch(
        &mut self,
        message: Message,
    ) -> Result<Option<LinuxWaylandLabEvent>, LinuxWaylandError> {
        if message.object == DISPLAY {
            return display_event(&message).map(|_| None);
        }
        if message.object == self.objects.registry {
            return registry_event(&message, None).map(|_| None);
        }
        if message.object == self.objects.shm {
            return shm_event(&message, None).map(|_| None);
        }
        if self.objects.seat == Some(message.object) {
            return seat_has_pointer(&message).map(|_| None);
        }
        if message.object == self.objects.xdg_wm_base {
            if message.opcode != 0 {
                return Err(LinuxWaylandError::ProtocolRejected);
            }
            let mut reader = message.reader();
            let serial = reader.uint().map_err(protocol_error)?;
            reader.finish().map_err(protocol_error)?;
            let mut pong = Request::new(self.objects.xdg_wm_base, 3);
            pong.uint(serial);
            self.send(pong)?;
            return Ok(None);
        }
        if message.object == self.objects.xdg_surface {
            if message.opcode != 0 {
                return Err(LinuxWaylandError::ProtocolRejected);
            }
            let mut reader = message.reader();
            let serial = reader.uint().map_err(protocol_error)?;
            reader.finish().map_err(protocol_error)?;
            let mut acknowledge = Request::new(self.objects.xdg_surface, 4);
            acknowledge.uint(serial);
            self.send(acknowledge)?;
            return Ok(None);
        }
        if message.object == self.objects.toplevel {
            return toplevel_closed(&message)
                .map(|closed| closed.then_some(LinuxWaylandLabEvent::Closed));
        }
        if self.objects.pointer == Some(message.object) {
            let pointer = self
                .pointer
                .as_mut()
                .ok_or(LinuxWaylandError::ProtocolRejected)?;
            return pointer
                .dispatch(&message, self.objects.surface)
                .map(|result| match result {
                    PointerResult::None => None,
                    PointerResult::Activated => Some(LinuxWaylandLabEvent::Activated),
                });
        }
        if let Some(buffer) = self
            .buffers
            .iter_mut()
            .find(|buffer| buffer.object == message.object)
        {
            if message.opcode != 0 || message.reader().finish().is_err() {
                return Err(LinuxWaylandError::ProtocolRejected);
            }
            buffer.release();
            return Ok(None);
        }
        Err(LinuxWaylandError::ProtocolRejected)
    }

    fn send(&self, request: Request) -> Result<(), LinuxWaylandError> {
        let bytes = request.finish().map_err(protocol_error)?;
        self.connection
            .send(&bytes)
            .map_err(|_| LinuxWaylandError::DesktopUnavailable)
    }

    fn next_message(&mut self) -> Result<Message, LinuxWaylandError> {
        next_message(&self.connection, &mut self.input)
    }

    fn next_message_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Message>, LinuxWaylandError> {
        next_message_with_timeout(&self.connection, &mut self.input, timeout)
    }
}

impl fmt::Debug for LinuxWaylandLab {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LinuxWaylandLab(..)")
    }
}

struct Objects {
    registry: u32,
    shm: u32,
    xdg_wm_base: u32,
    surface: u32,
    xdg_surface: u32,
    toplevel: u32,
    seat: Option<u32>,
    pointer: Option<u32>,
}

struct Setup {
    connection: Connection,
    input: Vec<u8>,
    ids: ObjectIds,
}

impl Setup {
    fn allocate(&mut self) -> Result<u32, LinuxWaylandError> {
        self.ids.allocate().map_err(protocol_error)
    }

    fn send(&self, request: Request) -> Result<(), LinuxWaylandError> {
        let bytes = request.finish().map_err(protocol_error)?;
        self.connection
            .send(&bytes)
            .map_err(|_| LinuxWaylandError::DesktopUnavailable)
    }

    fn bind(
        &self,
        global: Global,
        interface: &str,
        supported_version: u32,
        object: u32,
    ) -> Result<(), LinuxWaylandError> {
        let mut bind = Request::new(REGISTRY, 0);
        bind.uint(global.name);
        bind.string(interface).map_err(protocol_error)?;
        bind.uint(global.version.min(supported_version));
        bind.uint(object);
        self.send(bind)
    }

    fn wait_for_globals(
        &mut self,
        registry: u32,
        callback: u32,
    ) -> Result<Globals, LinuxWaylandError> {
        let mut globals = Globals::default();
        loop {
            let message = self.next_message()?;
            if message.object == DISPLAY {
                display_event(&message)?;
            } else if message.object == registry {
                registry_event(&message, Some(&mut globals))?;
            } else if message.object == callback && message.opcode == 0 {
                let mut reader = message.reader();
                reader.uint().map_err(protocol_error)?;
                reader.finish().map_err(protocol_error)?;
                return Ok(globals);
            } else {
                return Err(LinuxWaylandError::ProtocolRejected);
            }
        }
    }

    fn wait_for_formats_and_pointer(
        &mut self,
        registry: u32,
        shm: u32,
        seat: Option<u32>,
        callback: u32,
    ) -> Result<bool, LinuxWaylandError> {
        let mut xrgb_supported = false;
        let mut pointer_supported = false;
        loop {
            let message = self.next_message()?;
            if message.object == DISPLAY {
                display_event(&message)?;
            } else if message.object == registry {
                registry_event(&message, None)?;
            } else if message.object == shm {
                shm_event(&message, Some(&mut xrgb_supported))?;
            } else if seat == Some(message.object) {
                pointer_supported = seat_has_pointer(&message)?;
            } else if message.object == callback && message.opcode == 0 {
                let mut reader = message.reader();
                reader.uint().map_err(protocol_error)?;
                reader.finish().map_err(protocol_error)?;
                return xrgb_supported
                    .then_some(pointer_supported)
                    .ok_or(LinuxWaylandError::RequiredSupportUnavailable);
            } else {
                return Err(LinuxWaylandError::ProtocolRejected);
            }
        }
    }

    fn set_toplevel_metadata(&self, toplevel: u32) -> Result<(), LinuxWaylandError> {
        let mut title = Request::new(toplevel, 2);
        title.string("Anodrel Linux Lab").map_err(protocol_error)?;
        self.send(title)?;
        let mut app_id = Request::new(toplevel, 3);
        app_id
            .string("org.anodrel.development.lab")
            .map_err(protocol_error)?;
        self.send(app_id)?;
        for opcode in [7, 8] {
            let mut size = Request::new(toplevel, opcode);
            size.int(LAB_WIDTH as i32);
            size.int(LAB_HEIGHT as i32);
            self.send(size)?;
        }
        Ok(())
    }

    fn wait_for_initial_configure(&mut self, objects: &Objects) -> Result<(), LinuxWaylandError> {
        loop {
            let message = self.next_message()?;
            if message.object == DISPLAY {
                display_event(&message)?;
            } else if message.object == objects.xdg_wm_base && message.opcode == 0 {
                let mut reader = message.reader();
                let serial = reader.uint().map_err(protocol_error)?;
                reader.finish().map_err(protocol_error)?;
                let mut pong = Request::new(objects.xdg_wm_base, 3);
                pong.uint(serial);
                self.send(pong)?;
            } else if message.object == objects.xdg_surface && message.opcode == 0 {
                let mut reader = message.reader();
                let serial = reader.uint().map_err(protocol_error)?;
                reader.finish().map_err(protocol_error)?;
                let mut acknowledge = Request::new(objects.xdg_surface, 4);
                acknowledge.uint(serial);
                self.send(acknowledge)?;
                return Ok(());
            } else if message.object == objects.toplevel {
                if toplevel_closed(&message)? {
                    return Err(LinuxWaylandError::DesktopUnavailable);
                }
            } else {
                return Err(LinuxWaylandError::ProtocolRejected);
            }
        }
    }

    fn create_buffers(&mut self, shm: u32) -> Result<[Buffer; BUFFER_COUNT], LinuxWaylandError> {
        let byte_count = (LAB_WIDTH as usize)
            .checked_mul(LAB_HEIGHT as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(LinuxWaylandError::ProtocolRejected)?;
        let byte_count =
            i32::try_from(byte_count).map_err(|_| LinuxWaylandError::ProtocolRejected)?;
        let mut buffers = Vec::with_capacity(BUFFER_COUNT);
        for _ in 0..BUFFER_COUNT {
            let pool = self.allocate()?;
            let buffer_object = self.allocate()?;
            let mut memory = SharedMemory::create(byte_count as usize)
                .map_err(|_| LinuxWaylandError::DesktopUnavailable)?;
            let descriptor = memory
                .descriptor()
                .map_err(|_| LinuxWaylandError::DesktopUnavailable)?;
            let mut create_pool = Request::new(shm, 0);
            create_pool.uint(pool);
            create_pool.int(byte_count);
            let bytes = create_pool.finish().map_err(protocol_error)?;
            self.connection
                .send_descriptor(&bytes, descriptor)
                .map_err(|_| LinuxWaylandError::DesktopUnavailable)?;
            memory.close_descriptor();

            let mut create_buffer = Request::new(pool, 0);
            create_buffer.uint(buffer_object);
            create_buffer.int(0);
            create_buffer.int(LAB_WIDTH as i32);
            create_buffer.int(LAB_HEIGHT as i32);
            create_buffer.int((LAB_WIDTH * 4) as i32);
            create_buffer.uint(XRGB8888);
            self.send(create_buffer)?;
            self.send(Request::new(pool, 1))?;
            buffers.push(Buffer::new(buffer_object, memory));
        }
        buffers
            .try_into()
            .map_err(|_| LinuxWaylandError::ProtocolRejected)
    }

    fn next_message(&mut self) -> Result<Message, LinuxWaylandError> {
        next_message(&self.connection, &mut self.input)
    }
}

fn new_id_request(object: u32, opcode: u16, id: u32) -> Request {
    let mut request = Request::new(object, opcode);
    request.uint(id);
    request
}

#[cfg(test)]
mod tests {
    use super::{LAB_HEIGHT, LAB_WIDTH, LinuxWaylandError};

    #[test]
    fn fixed_lab_extent_fits_a_signed_wayland_int() {
        assert!(LAB_WIDTH <= i32::MAX as u32);
        assert!(LAB_HEIGHT <= i32::MAX as u32);
        assert_eq!(
            LinuxWaylandError::Backpressured.to_string(),
            "Linux compositor has not released a frame buffer"
        );
    }
}
