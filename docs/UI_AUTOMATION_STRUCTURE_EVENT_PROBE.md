# Anodrel Windows UI Automation structure-event probe

## Purpose

`--uia-structure-event-probe <native-client.exe>` is a development-only Windows
acceptance diagnostic. It runs one compiled two-document native child through
the ordinary three-grant authenticated session, then verifies one real
`ChildrenInvalidated` event from the session window's fixed `anodrel.surface`
root.

The child publishes its compiled initial document at revision 1 and waits for
only `native.structure.prepare`. A private direct Windows UI Automation client
selects and prepares that one standard Invoke interface, registers one
element-scoped structure listener on the root, arms it, and invokes the
prepared action once. The child then receives that revision-1 action, publishes
its compiled replacement document at revision 2, and waits only for
`native.structure.complete`.

The probe passes only when Windows delivers `ChildrenInvalidated` from the
fixed root. It removes the listener, uses a fresh private client to invoke the
fixed complete action, and passes only after the child receives its revision-2
action and closes its own session. This proves the authenticated replacement,
provider event, Windows callback, bounded semantic mailbox, and normal child
close sequence together.

## Running it

From the repository root on Windows:

~~~text
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-structure-event-client
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --uia-structure-event-probe native/target/release/anodrel-native-structure-event-client.exe
~~~

Or double-click `start-uia-structure-event-probe.bat` in the repository root.

The temporary authenticated window may appear briefly and closes after the
compiled child consumes its second fixed action. A successful run prints `UI
Automation structure-event probe passed.` A failure closes only that
development session, terminates its selected child, and exits non-zero.

## Boundary

This route is private to the Windows development host. It uses only direct
Ole32 and Windows UI Automation client APIs; it ships no browser, webview, test
framework, or third-party runtime binding.

The probe accepts one development executable path but no document, element ID,
selector, coordinate, action value, event choice, event data, callback, or
application input. The child has no listener or readiness channel. The host
owns the fixed documents, two fixed actions, root source, event kind, handler,
and outcome.

Decision 0076 fixes Anodrel's `UiaRaiseStructureChangedEvent` input to a null
runtime-ID pointer and zero length; a unit test protects that provider call.
The UI Automation callback's runtime-ID representation is Windows-owned and
the probe does not read it or infer provider input from it. The direct
acceptance assertion is the fixed event source and `ChildrenInvalidated` kind.

The probe supplements the manual checks in
`docs/ACCESSIBILITY_VERIFICATION.md`; it does not prove Narrator speech,
Inspect-highlight correctness, repeated or rejected-revision behavior,
arbitrary event subscriptions, or application behaviour beyond this compiled
diagnostic. See Decisions 0076 and 0114.
