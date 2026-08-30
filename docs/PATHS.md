# Application directories v1

**Status:** Windows and Linux host foundation. This is a host-owned
directory-layout contract, not a public filesystem capability or protocol
operation.

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

`anodrel-linux-paths` obtains only the effective Linux account's home directory
through direct geteuid and getpwuid_r calls, then derives its fixed default
local-data root. It does not read HOME, XDG variables, the current working
directory, or application input.

## Layout

For an application ID such as `org.anodrel.sample`, Windows resolves one
current-user root and derives these locations:

~~~text
%LOCALAPPDATA%\Anodrel\Applications\org.anodrel.sample\
|- data\
|- cache\
`- logs\
~~~

The host also has locations of its own, in a namespace beside `Applications`
rather than inside it:

~~~text
%LOCALAPPDATA%\Anodrel\Host\
`- logs\
~~~

`HostDirectories` derives these from the same root and takes no identity,
because what goes here belongs to no application. A host defect filed under
whichever application happened to be loaded would be misattributed, and would
put one application's evidence where another application's host will look. See
`docs/CRASH_REPORTS.md` and Decision 0065 for the first use.

Because `Host` is a sibling of `Applications`, no application identity can
resolve to it. A unit test asserts that rather than leaving it to the identity
grammar.

## Linux root and layout

Linux derives its local-data root from the effective account record rather than
an inherited environment variable:

~~~text
<effective account home>/.local/share
~~~

It then retains the same stable Anodrel namespace:

~~~text
<effective account home>/.local/share/Anodrel/Applications/org.anodrel.sample/
|- data/
|- cache/
\\-- logs/
~~~

Host logs remain under the sibling Host namespace. The current adapter
intentionally does not interpret XDG_DATA_HOME; that convention is a later
configuration and migration decision, not an ambient application input.

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
and the three leaf names are stable once data is written there, as is
`Anodrel\Host`. A compatible extension may add a new named location; the host
namespace was added that way. Renaming a location, changing the root, or
changing an application-ID-to-directory mapping needs an explicit migration and
a new documented version.

No `platform.paths` operation exists in Protocol v1. Until a host exposes a
documented storage or logging capability, these absolute paths remain native
host values and must not appear in rendered content, protocol diagnostics, or
the typed diagnostic log.

## Failure and performance behavior

The Windows adapter fails closed if Windows cannot provide Local AppData, the
returned UTF-16 path is malformed, or the portable layout rejects its root or
identity. The Linux adapter does the same if its effective account home cannot
be read or cannot form an absolute local-data root. Errors do not carry an
absolute path, native status code, UID, account name, or user-profile
information.

One Windows lookup makes one known-folder operating-system call; one Linux
lookup makes one bounded reentrant account-record call. Both add only a few
fixed path joins. Neither enumerates or opens an Anodrel directory, and neither
retains a global cache; the host owns caching if it needs a longer-lived policy.
Linux account-service lookup remains an operating-system concern and may use
the account sources configured by that machine.

## Verification

Unit tests cover identity validation, absolute-root enforcement, exact layout,
and the no-creation guarantee. A Windows adapter test reads the current user's
known folder without mutating it. Run:

~~~text
cargo test --manifest-path native/Cargo.toml -p anodrel-paths
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-paths
wsl -- bash -lc 'source "$HOME/.cargo/env" && cd "/mnt/c/Users/Owner/Desktop/Platform X/native" && CARGO_TARGET_DIR=/tmp/anodrel-linux-target cargo test -p anodrel-linux-paths'
~~~

Decisions 0021 and 0124 record the namespace and Linux root choices.
