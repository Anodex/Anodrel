use std::io::Read;

use anodrel_bootstrap::BootstrapInvitation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_path = std::env::args()
        .nth(1)
        .ok_or("fixture requires a non-secret output path")?;
    let mut stdin = std::io::stdin().lock();
    let invitation = BootstrapInvitation::read_from(&mut stdin)?;
    let mut extra = [0_u8; 1];
    if stdin.read(&mut extra)? != 0 {
        return Err("bootstrap channel unexpectedly contained extra data".into());
    }
    std::fs::write(
        output_path,
        format!("{}\n{}\n", invitation.pipe_name(), invitation.session_id()),
    )?;
    Ok(())
}
