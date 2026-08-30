# 0124 — Linux application paths use the effective account home

- Status: Accepted
- Date: 2026-08-30

## Context

Anodrel already derives application data, cache, and host-log locations from a
host-selected current-user root on Windows. A Linux host needs the same
deterministic, no-Anodrel-directory-I/O layout foundation before it can attach
storage, logging, or a desktop surface.

The process environment is mutable ambient state. In particular, HOME and XDG
variables can be absent, relative, malformed, or deliberately supplied by a
parent process. Letting a child or application choose the root would undermine
the identity-bound layout and make the resulting directory policy hard to
audit.

## Decision

Add a Linux-only paths adapter. It reads the current process effective UID with
geteuid, resolves that account through the reentrant getpwuid_r Linux C-library
interface, and obtains the account home directory from the returned record. It
then derives the direct Linux local-data root as:

~~~text
<effective-account-home>/.local/share
~~~

The adapter rejects an unavailable account, malformed account record, missing
home value, or relative root. It does not read HOME, XDG_DATA_HOME, the current
working directory, command arguments, application data, or a configuration
file. It creates, enumerates, deletes, opens, watches, or exposes no directory.

The existing portable layout keeps its stable namespace below that root:

~~~text
<home>/.local/share/Anodrel/Applications/<validated application ID>/{data,cache,logs}
<home>/.local/share/Anodrel/Host/logs
~~~

The lookup returns only the existing portable directory values. Their Debug
forms remain path-free, and its safe failure categories never contain an
absolute path, account name, UID, or Linux status code.

## Consequences

- Linux gains a direct first-party current-user path foundation using its
  account interfaces and the Rust standard library only.
- The Linux layout is deterministic even when inherited environment variables
  are surprising or hostile.
- The data root follows the conventional default XDG data location but does not
  implement configurable XDG environment semantics yet.
- Application storage, logging, package discovery, installation, migration, and
  a filesystem capability remain separate decisions.

## Alternatives considered

**Trust HOME.** It is inherited mutable process state and can be missing or
relative. Refused.

**Use XDG_DATA_HOME directly.** It adds a configuration and normalization
policy before Anodrel has a Linux installer or migration story. Refused.

**Use a fixed root such as /var/lib.** It changes ownership and deployment
requirements from current-user host state to a system service. Refused.

**Add a third-party directory library.** It would add a shipped dependency for
a short direct account lookup. Refused.

## Revisit conditions

Revisit before supporting explicit XDG configuration, sandbox portals,
system-service paths, installation, migration from a prior root, a Linux
storage or logging writer, another Unix implementation, or an application
filesystem capability.
