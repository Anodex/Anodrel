# Application directories v1

**Status:** Windows host foundation. This is a host-owned directory-layout
contract, not a public filesystem capability or protocol operation.

## Purpose and boundary

Applications need stable locations for their own durable data, disposable
cache, and host-managed logs. They must not choose an absolute path, infer a
shared working directory, or receive another application's location.

`anodrel-paths` defines the portable layout from a validated application ID.
`anodrel-windows-paths` obtains only the current user's Windows Local AppData
known folder through `SHGetKnownFolderPath`. The adapter does not read,
create, enumerate, delete, watch, or expose any directory to the application
protocol. A later storage or logging service must own its own permissions,
creation behavior, and public contract.

## Layout

For an application ID such as `org.anodrel.sample`, Windows resolves one
current-user root and derives these locations:

~~~text
%LOCALAPPDATA%\Anodrel\Applications\org.anodrel.sample\
|- data\
|- cache\
`- logs\
~~~

The portable layout builder accepts only the existing validated application-ID
grammar: 3 to 128 lowercase ASCII letters, digits, `.`, `-`, or `_`, beginning
and ending with a letter or digit. It rejects a relative operating-system root
and invalid identity before it joins any path. The fixed path components are
not configurable by a package, request, environment value, rendered content,
or command-line child.

Each returned path is a location, not a promise that a directory exists. The
lookup performs no filesystem mutation. A future writer must create a required
directory with an operation-specific containment and recovery policy.

## Compatibility

This is layout version 1. The `Anodrel\Applications\<applicationId>` namespace
and the three leaf names are stable once data is written there. A compatible
extension may add a new named location. Renaming a location, changing the root,
or changing an application-ID-to-directory mapping needs an explicit migration
and a new documented version.

No `platform.paths` operation exists in Protocol v1. Until a host exposes a
documented storage or logging capability, these absolute paths remain native
host values and must not appear in rendered content, protocol diagnostics, or
the typed diagnostic log.

## Failure and performance behavior

The Windows adapter fails closed if Windows cannot provide Local AppData, the
returned UTF-16 path is malformed, or the portable layout rejects its root or
identity. Errors do not carry an absolute path, native status code, or user
profile information.

One lookup makes one known-folder operating-system call and a few fixed path
joins. It does not enumerate a directory tree, perform disk I/O, or retain a
global cache; the host owns caching if it needs a longer-lived policy.

## Verification

Unit tests cover identity validation, absolute-root enforcement, exact layout,
and the no-creation guarantee. A Windows adapter test reads the current user's
known folder without mutating it. Run:

~~~text
cargo test --manifest-path native/Cargo.toml -p anodrel-paths
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-paths
~~~

Decision 0021 records the namespace and ownership choice.
