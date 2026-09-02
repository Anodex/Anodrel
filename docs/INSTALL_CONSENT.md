# Windows initial-install consent

**Status:** The host-owned native confirmation is implemented. The later fixed
UAC handoff, completion proof, restart, and completion presentation remain
separate work.

## Purpose

Before an Anodrel installer asks Windows to elevate an initial installation, a
person using the machine must make a local decision. The confirmation accepts
only an opaque `PreparedInitialInstall` that has already proved the current
installer is signed and that no selected machine policy exists. It shows this
fixed native confirmation:

~~~text
Install Anodrel version <signed version> for all users?
~~~

The title is `Anodrel installer`. The version comes only from the current
signed embedded release. No application text, identity, path, publisher,
certificate, capability, package detail, or installation location is shown.

## Native behavior

The adapter calls the direct Windows `MessageBoxW` API with an information
icon, `Yes` and `No` buttons, and `No` as the focused default. It has no owner
window in this first console-installer slice. The caller must invoke it on its
native UI thread after the signed preflight, not from application protocol
code.

`Yes` creates an opaque approval that retains the original prepared
installation. Only that approval may enter the later UAC-handoff boundary.
`No` is an ordinary decline. The adapter does not download, elevate, launch,
install, write policy, remember a preference, or retain a decision.

## Exclusions

This is not an application-controlled dialog, a protocol operation, a default
installer command, a custom installer window, a UAC replacement, a settings
preference, a background prompt, a progress display, a restart request, or a
completion notification.

See [initial-install acceptance](INSTALL_ACCEPTANCE.md), [Windows installer](WINDOWS_INSTALLER.md),
and Decision 0178.
