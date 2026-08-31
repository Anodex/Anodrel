# Windows capability bridge

**Status:** Implemented for the internal Windows registered product session.
This document describes host composition, not a public application API.

## Purpose

Anodrel does not give a connected application ambient operating-system access.
A machine-selected installed-application record supplies a fixed capability
policy. Before the peer can authenticate, the Windows host derives that policy
and constructs the matching fixed service bundle.

~~~text
machine policy record
        |
installed-record validation and selected capabilities
        |
        +--> fixed HostPolicy ------> authenticated pipe session
        |
        +--> fixed HostServices ----> authenticated pipe session
~~~

The application receives protocol operations only after the pipe session has
been constructed with both values. A protocol request must pass its capability
check before the corresponding host service is reached.

## Windows composition

`anodrel-windows-registered-session` reads one application record from the
machine policy store, validates it, and derives its `HostPolicy`. It begins with
unavailable services, then installs direct Windows adapters and host-owned UI
mailboxes into one `HostServices` value before it creates the named-pipe
session. That value cannot be altered by the connected application.

| Service area | Windows composition |
| --- | --- |
| Clipboard, external links, state, credentials | Identity-bound Windows adapters |
| Network | Available only for machine-policy `network.fetch` origins |
| File and folder access | Host-owned picker mailboxes and retained-object services |
| Notifications, menus, and window controls | UI-thread bridges owned by the host |
| UI documents, input, fields, and windows | One grouped native UI resource set |

The product-session coordinator joins this registered session with a
digest-revalidated, signer-verified child launch and its authenticated native
window. It does not add a second, broader authority path.

## Security properties

- The application cannot choose or modify its record, capabilities, service
  implementations, policy directory, executable, or launch arguments.
- An unsupported service starts unavailable; a capability name alone does not
  create an operating-system handle.
- Network permission is constrained to exact machine-policy origins rather
  than application-supplied URLs.
- UI-affine resources remain on the Windows UI thread; pipe work stays off it.
- The protocol has no operation that mutates the policy or service bundle after
  the session starts.

## Boundaries that remain

The bridge is an internal Windows runtime boundary, not a release mechanism.
Production publisher trust, packaging, installation, updates, multi-window
policy, restart policy, and background execution remain separate Windows
release gates.

## Verification

- Core service tests prove that capability checks happen before service use.
- Registered-session tests cover policy-derived fixed service composition before
  a peer connects.
- Product-session tests cover joining that session to verified child launch and
  the native-window lifetime.

See [installed application records](LAUNCH.md), [product sessions](PRODUCT_SESSIONS.md),
[Windows release readiness](WINDOWS_RELEASE.md), and Decisions 0018 and 0060.
