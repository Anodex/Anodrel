# Windows Start-menu registration

**Status:** The signed selected-policy preflight, version 1.4 release metadata,
matching record v1.23, direct Shell Link writer, fixed launcher route, and
policy-transaction composition are implemented. Apps & features and AUMID
registration remain separate work.

## Purpose

An all-users Start-menu shortcut must name and launch the same selected release
as Windows machine policy. It cannot accept product text, an executable path,
or a shortcut location from an application or command line. Before the writer
can run, its preflight verifies the current signed installer, reads only that
installer identity's selected record, validates the selected child and launcher
Authenticode signers against both policy and the installer, and requires record
v1.23's signed product display metadata, Start-menu filename, and launcher.

## Fixed target

The direct Windows writer derives exactly one all-users link:

~~~text
Common Programs\Anodrel\<signed-start-menu-name>.lnk
~~~

The link's target, one generated argument sequence, and visible filename come
only from the verified selected record. `startMenuName` is a separately signed, Windows-safe file component;
the general product display name is never used in a filename, directory,
registry key, command, URL, or authority decision.

## Preflight boundary

The opaque preflight result has no application protocol, user input, path,
shortcut filename, icon, working directory, generated arguments, registry-write,
COM-object, link, launch, notification, or removal operation. A record through
v1.22 has no product launcher and therefore refuses registration rather than
inventing a direct-child target.

## Native link write

`refresh_current_product_shortcut` repeats the full selected-policy proof
immediately before its one Shell Link operation. It resolves `FOLDERID_CommonPrograms`
directly, requires that directory and its fixed `Anodrel` child to be ordinary
non-reparse directories, and writes the link through a temporary ordinary file
followed by same-directory replacement with write-through. The link's target is
the selected product launcher, its working directory is the selected package
root, and its only argument sequence is generated as
`--product-launch <selected application ID>`. It has no custom icon,
description, source URL, application input, or runtime path discovery.

The direct writer has an automated Windows test that creates a link only inside
a temporary directory and removes it afterwards. It also reads the persisted
arguments back and proves the fixed product-launch sequence. Its product-facing
route still requires a signed v1.4 installer and selected v1.23 machine record.

## Policy-transition composition

Install, update, and rollback synchronize the product link only after the
machine record has selected the resulting complete release. Update and rollback
capture only the previously selected optional signed name before they change
policy. If the new selected release has a different name, the new link is
persisted first and the old regular link is removed afterwards. A selected
record without a launcher removes its prior link rather than presenting an
invalid direct-child entry point.

If that post-policy work fails, the selected machine record remains valid and
the installer reports incomplete registration; it does not roll policy back.
Uninstall removes a currently verified regular link before it removes policy or
package files. A missing link is harmless, while an unsafe or undeletable link
leaves the selected policy untouched.

## Exclusions

This does not register an Application User Model ID, write Apps & features data,
add a taskbar pin, create a desktop shortcut, choose a
custom icon, accept command-line arguments, launch an application, or report
whether a person saw a Start-menu item.

See [product registration](PRODUCT_REGISTRATION.md), [installed application records](LAUNCH.md),
and Decisions 0183 through 0187.
