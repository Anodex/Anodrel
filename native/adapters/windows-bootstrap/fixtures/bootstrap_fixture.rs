use std::io::Read;

use anodrel_bootstrap::BootstrapInvitation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let output_path = arguments
        .next()
        .ok_or("fixture requires a non-secret output path")?;
    let wait_for_termination = matches!(arguments.next().as_deref(), Some("--wait"));
    if arguments.next().is_some() {
        return Err("fixture received an unexpected argument".into());
    }
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
    if wait_for_termination {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    Ok(())
}
