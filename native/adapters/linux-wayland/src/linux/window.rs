//! Fixed Wayland diagnostic-window lifecycle.

use std::fmt;

use anodrel_canvas::Canvas;

use super::{
    buffer::{BUFFER_COUNT, Buffer},
    error::LinuxWaylandError,
    globals::{Global, Globals},
    locator::Locator,
    raw::{Connection, SharedMemory},
    wire::{Message, ObjectIds, Request, WireError},
};

/// The one fixed diagnostic canvas width.
pub const LAB_WIDTH: u32 = 960;
/// The one fixed diagnostic canvas height.
pub const LAB_HEIGHT: u32 = 640;

const DISPLAY: u32 = 1;
const REGISTRY: u32 = 2;
const XRGB8888: u32 = 1;
const COMPOSITOR_VERSION: u32 = 1;

/// One fixed direct-Wayland development diagnostic with no window API surface.
pub struct LinuxWaylandLab {
    connection: Connection,
    input: Vec<u8>,
    objects: Objects,
    buffers: [Buffer; BUFFER_COUNT],
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

        let format_sync = setup.allocate()?;
        setup.send(new_id_request(DISPLAY, 0, format_sync))?;
        setup.wait_for_formats(registry, shm, format_sync)?;

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
        };
        setup.wait_for_initial_configure(&objects)?;
        let buffers = setup.create_buffers(shm)?;
        Ok(Self {
            connection: setup.connection,
            input: setup.input,
            objects,
            buffers,
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
            let message = self.next_message()?;
            if self.dispatch(message)? {
                return Ok(());
            }
        }
    }

    fn dispatch(&mut self, message: Message) -> Result<bool, LinuxWaylandError> {
        if message.object == DISPLAY {
            return display_event(&message).map(|_| false);
        }
        if message.object == self.objects.registry {
            return registry_event(&message, None).map(|_| false);
        }
        if message.object == self.objects.shm {
            return shm_event(&message, None).map(|_| false);
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
            return Ok(false);
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
            return Ok(false);
        }
        if message.object == self.objects.toplevel {
            return toplevel_event(&message);
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
            return Ok(false);
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

    fn wait_for_formats(
        &mut self,
        registry: u32,
        shm: u32,
        callback: u32,
    ) -> Result<(), LinuxWaylandError> {
        let mut xrgb_supported = false;
        loop {
            let message = self.next_message()?;
            if message.object == DISPLAY {
                display_event(&message)?;
            } else if message.object == registry {
                registry_event(&message, None)?;
            } else if message.object == shm {
                shm_event(&message, Some(&mut xrgb_supported))?;
            } else if message.object == callback && message.opcode == 0 {
                let mut reader = message.reader();
                reader.uint().map_err(protocol_error)?;
                reader.finish().map_err(protocol_error)?;
                return xrgb_supported
                    .then_some(())
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
                if toplevel_event(&message)? {
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

fn next_message(
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

fn display_event(message: &Message) -> Result<(), LinuxWaylandError> {
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

fn registry_event(
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

fn shm_event(message: &Message, xrgb: Option<&mut bool>) -> Result<(), LinuxWaylandError> {
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

fn toplevel_event(message: &Message) -> Result<bool, LinuxWaylandError> {
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

fn protocol_error(_: WireError) -> LinuxWaylandError {
    LinuxWaylandError::ProtocolRejected
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
