# Windows Start-menu registration

**Status:** The signed selected-policy preflight, version 1.3 release metadata,
matching record v1.22, and direct Shell Link writer are implemented. Installer
composition after install, update, rollback, and uninstall remains separate
work.

## Purpose

An all-users Start-menu shortcut must name and launch the same selected release
as Windows machine policy. It cannot accept product text, an executable path,
or a shortcut location from an application or command line. Before a future
writer can run, its preflight verifies the current signed installer, reads only
that installer identity's selected record, validates the selected executable's
Authenticode signer against both policy and the installer, and requires record
v1.22's signed product display metadata and Start-menu filename.

## Planned fixed target

The direct Windows writer derives exactly one all-users link:

~~~text
Common Programs\Anodrel\<signed-start-menu-name>.lnk
~~~

The link's target and visible filename come only from the verified selected
record. `startMenuName` is a separately signed, Windows-safe file component;
the general product display name is never used in a filename, directory,
registry key, command, URL, or authority decision.

## Preflight boundary

The opaque preflight result has no application protocol, user input, path,
shortcut filename, icon, working directory, arguments, registry-write,
COM-object, link, launch, notification, or removal operation. A legacy record
without v1.22 Start-menu metadata refuses registration rather than inventing a
label.

## Native link write

`refresh_current_product_shortcut` repeats the full selected-policy proof
immediately before its one Shell Link operation. It resolves `FOLDERID_CommonPrograms`
directly, requires that directory and its fixed `Anodrel` child to be ordinary
non-reparse directories, and writes the link through a temporary ordinary file
followed by same-directory replacement with write-through. The link's target is
the selected executable and its working directory is the selected package root.
It has no argument, custom icon, description, source URL, application input,
or runtime path discovery.

The direct writer has an automated Windows test that creates a link only inside
a temporary directory and removes it afterwards. Its product-facing
route still requires a signed v1.3 installer and selected v1.22 machine record.

## Exclusions

This does not yet remove a link, register an Application User Model ID, write
Apps & features data, add a taskbar pin, create a desktop shortcut, choose a
custom icon, pass command-line arguments, launch an application, or report
whether a person saw a Start-menu item.

See [product registration](PRODUCT_REGISTRATION.md), [installed application records](LAUNCH.md),
and Decisions 0183 through 0185.
