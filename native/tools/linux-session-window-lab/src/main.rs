//! Fixed Linux child/view lifetime check for the Anodrel Linux Session Lab.

#[cfg(all(target_os = "linux", target_endian = "little"))]
mod linux;

#[cfg(all(target_os = "linux", target_endian = "little"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run()
}

#[cfg(not(all(target_os = "linux", target_endian = "little")))]
fn main() {
    eprintln!("The Anodrel Linux Session Lab requires little-endian Linux.");
}
