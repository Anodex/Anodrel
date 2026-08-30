//! Bounded global discovery for the fixed Linux Lab interfaces.

#[derive(Clone, Copy)]
pub(super) struct Global {
    pub(super) name: u32,
    pub(super) version: u32,
}

#[derive(Default)]
pub(super) struct Globals {
    compositor: Option<Global>,
    shm: Option<Global>,
    xdg_wm_base: Option<Global>,
    seat: Option<Global>,
}

impl Globals {
    pub(super) fn record(&mut self, name: u32, interface: &str, version: u32) {
        let slot = match interface {
            "wl_compositor" => &mut self.compositor,
            "wl_shm" => &mut self.shm,
            "xdg_wm_base" => &mut self.xdg_wm_base,
            "wl_seat" => &mut self.seat,
            _ => return,
        };
        if slot.is_none() {
            *slot = Some(Global { name, version });
        }
    }

    pub(super) fn compositor(&self) -> Option<Global> {
        self.compositor.filter(|global| global.version >= 1)
    }

    pub(super) fn shm(&self) -> Option<Global> {
        self.shm.filter(|global| global.version >= 1)
    }

    pub(super) fn xdg_wm_base(&self) -> Option<Global> {
        self.xdg_wm_base.filter(|global| global.version >= 1)
    }

    pub(super) fn seat(&self) -> Option<Global> {
        self.seat.filter(|global| global.version >= 1)
    }
}

#[cfg(test)]
mod tests {
    use super::Globals;

    #[test]
    fn keeps_the_first_global_and_accepts_the_base_compositor_version() {
        let mut globals = Globals::default();
        globals.record(7, "wl_compositor", 1);
        globals.record(9, "wl_compositor", 5);
        globals.record(11, "wl_shm", 1);
        globals.record(13, "xdg_wm_base", 6);
        globals.record(15, "wl_seat", 8);

        assert_eq!(globals.compositor().map(|global| global.name), Some(7));
        assert_eq!(globals.shm().map(|global| global.name), Some(11));
        assert_eq!(globals.xdg_wm_base().map(|global| global.name), Some(13));
        assert_eq!(globals.seat().map(|global| global.name), Some(15));
    }
}
