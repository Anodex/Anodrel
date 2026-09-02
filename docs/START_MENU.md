# Windows Start-menu registration

**Status:** The signed selected-policy preflight is implemented. The direct
Shell Link writer and installer composition remain separate work.

## Purpose

An all-users Start-menu shortcut must name and launch the same selected release
as Windows machine policy. It cannot accept product text, an executable path,
or a shortcut location from an application or command line. Before a future
writer can run, its preflight verifies the current signed installer, reads only
that installer identity's selected record, validates the selected executable's
Authenticode signer against both policy and the installer, and requires record
v1.21's signed product display metadata.

## Planned fixed target

The later direct Windows writer will derive exactly one all-users link:

~~~text
Common Programs\Anodrel\<application-id>.lnk
~~~

The application ID is a safe stable machine-policy key, not product-facing
text. The link's target and visible display values come only from the verified
selected record. The display name is never used in a filename, directory,
registry key, command, URL, or authority decision.

## Preflight boundary

The opaque preflight result has no application protocol, user input, path,
shortcut filename, icon, working directory, arguments, registry-write,
COM-object, link, launch, notification, or removal operation. A legacy record
without v1.21 product metadata refuses registration rather than inventing a
label.

## Exclusions

This does not yet create or remove a link, register an Application User Model
ID, write Apps & features data, add a taskbar pin, create a desktop shortcut,
choose an icon, pass command-line arguments, launch an application, or report
whether a person saw a Start-menu item.

See [product registration](PRODUCT_REGISTRATION.md), [installed application records](LAUNCH.md),
and Decision 0183.
