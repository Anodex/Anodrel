//! Fixed best-effort destructor order for one Wayland Lab connection.

use super::{
    buffer::{BUFFER_COUNT, Buffer},
    raw::Connection,
    window::Objects,
    wire::Request,
};

const TEARDOWN_REQUEST_COUNT: usize = BUFFER_COUNT + 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TeardownRequest {
    object: u32,
    opcode: u16,
}

pub(super) fn close(connection: &Connection, objects: &Objects, buffers: &[Buffer; BUFFER_COUNT]) {
    let buffer_objects = [buffers[0].object, buffers[1].object];
    for request in teardown_plan(objects, buffer_objects) {
        let Ok(bytes) = Request::new(request.object, request.opcode).finish() else {
            continue;
        };
        let _ = connection.send(&bytes);
    }
}

fn teardown_plan(
    objects: &Objects,
    buffers: [u32; BUFFER_COUNT],
) -> [TeardownRequest; TEARDOWN_REQUEST_COUNT] {
    [
        TeardownRequest {
            object: objects.toplevel,
            opcode: 0,
        },
        TeardownRequest {
            object: objects.xdg_surface,
            opcode: 0,
        },
        TeardownRequest {
            object: objects.surface,
            opcode: 0,
        },
        TeardownRequest {
            object: buffers[0],
            opcode: 0,
        },
        TeardownRequest {
            object: buffers[1],
            opcode: 0,
        },
        TeardownRequest {
            object: objects.xdg_wm_base,
            opcode: 0,
        },
        TeardownRequest {
            object: objects.shm,
            opcode: 1,
        },
        TeardownRequest {
            object: objects.compositor,
            opcode: 0,
        },
        TeardownRequest {
            object: objects.registry,
            opcode: 1,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{Objects, TeardownRequest, teardown_plan};

    #[test]
    fn teardown_destroys_fixed_children_before_their_parent_globals() {
        let objects = Objects {
            registry: 2,
            compositor: 3,
            shm: 4,
            xdg_wm_base: 5,
            surface: 6,
            xdg_surface: 7,
            toplevel: 8,
            seat: Some(9),
            pointer: Some(10),
        };
        let plan = teardown_plan(&objects, [11, 12]);
        assert_eq!(
            plan,
            [
                TeardownRequest {
                    object: 8,
                    opcode: 0
                },
                TeardownRequest {
                    object: 7,
                    opcode: 0
                },
                TeardownRequest {
                    object: 6,
                    opcode: 0
                },
                TeardownRequest {
                    object: 11,
                    opcode: 0
                },
                TeardownRequest {
                    object: 12,
                    opcode: 0
                },
                TeardownRequest {
                    object: 5,
                    opcode: 0
                },
                TeardownRequest {
                    object: 4,
                    opcode: 1
                },
                TeardownRequest {
                    object: 3,
                    opcode: 0
                },
                TeardownRequest {
                    object: 2,
                    opcode: 1
                },
            ]
        );
    }
}
