//! Linux-only direct Wayland presentation check for the Anodrel Linux Lab.

use std::error::Error;

use anodrel_linux_lab_surface::compose;
use anodrel_linux_wayland::{LinuxWaylandLab, LinuxWaylandLabEvent};

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let mut lab = LinuxWaylandLab::open()?;
    let canvas = compose(false);
    lab.present(&canvas)?;
    let mut activated = false;
    loop {
        match lab.wait_for_lab_event()? {
            LinuxWaylandLabEvent::Activated if !activated => {
                lab.present(&compose(true))?;
                activated = true;
            }
            LinuxWaylandLabEvent::Activated => {}
            LinuxWaylandLabEvent::Closed => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_linux_lab_surface::{LAB_HEIGHT, LAB_WIDTH, compose};

    #[test]
    fn fixed_surface_keeps_the_wayland_lab_extent() {
        let canvas = compose(false);
        assert_eq!((canvas.width(), canvas.height()), (LAB_WIDTH, LAB_HEIGHT));
    }
}
