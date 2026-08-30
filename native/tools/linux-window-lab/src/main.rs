//! First direct Wayland presentation check for the Anodrel Linux Lab.

#[cfg(all(target_os = "linux", target_endian = "little"))]
mod linux;

#[cfg(all(target_os = "linux", target_endian = "little"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run()
}

#[cfg(not(all(target_os = "linux", target_endian = "little")))]
fn main() {
    eprintln!("The Anodrel Linux Wayland Lab requires little-endian Linux.");
}
