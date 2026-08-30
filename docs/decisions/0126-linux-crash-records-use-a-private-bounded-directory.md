# Decision 0126: Linux crash records use a private bounded directory

- Status: Accepted
- Date: 2026-08-30

## Context

`anodrel-crash` already defines one closed, host-only record format for a
contained panic. The Windows store writes at most eight records below the
host-owned log location. Linux now has the same effective-account directory
foundation and a direct private state store, but it has no crash-record writer.

A crash reporter runs while the host is ending after a defect. A generic
logging API, a retry loop, an application-readable file, an unbounded directory
scan, or arbitrary crash text would all make that failure path less safe and
less honest.

## Decision

Add a Linux-only `CrashReporter` adapter. It derives the host log location from
the effective-account root through `anodrel-linux-paths`:

~~~text
<effective-account-home>/.local/share/Anodrel/Host/logs
~~~

It opens the existing account home component by component from `/` with Linux
`open` and `openat` using `O_DIRECTORY`, `O_NOFOLLOW`, and `O_CLOEXEC`. It
creates only the `Anodrel/Host/logs` portion beneath the account home. Every
created or accepted Anodrel component must be owned by the effective account
and private to it with mode 0700; `.local/share` is placement, not an Anodrel
ownership claim.

The store enumerates at most 128 immediate directory entries through the opened
log-directory descriptor and considers at most 64 private regular candidate
names. It recognises only
`crash-<canonical decimal sequence>.anodrel.v1`. A directory, symbolic link,
hard link, foreign-owned or permissive file, malformed name, non-UTF-8 name,
or unrelated file is ignored; it is not read, reported, or deleted. Reaching
the candidate bound returns the existing safe write failure rather than
performing an unbounded scan.

The next sequence is one greater than the highest recognised record. The store
serializes the existing closed portable record, creates only its fixed generated
name with `O_CREAT|O_EXCL` and mode 0600, writes and synchronizes its bounded
contents, then synchronizes the directory. It removes oldest recognised
records only after the new record exists and only as best effort until the
eight-record retention policy is met. It never retries, chooses another
location, logs a native failure, or exposes any result through a protocol.

An in-process crash-store call takes one non-blocking operation claim.
Coordination between multiple host processes is not implied; a simultaneous
call or exclusive-name collision is a safe failed report.

## Consequences

- Linux gains direct host crash evidence without a Linux desktop host, a
  general diagnostic-log service, or any application-visible capability.
- The existing portable format and error categories stay unchanged across
  Windows and Linux.
- Record creation performs one bounded descriptor walk and a fixed-size write.
  First use also creates the short private directory tail.
- Stray files cannot become records or be deleted by retention. An
  overpopulated hostile-looking directory fails safely instead of extending
  shutdown work without bound.

## Alternatives considered

**Use a path-based directory scan.** It would reopen names after discovery and
would not preserve the no-follow ownership checks. Refused.

**Add the record to `LogBook`.** That ledger is authenticated-application
readable and intentionally process-local. Refused by Decision 0065.

**Send a report to a service or remote endpoint.** It would add network,
identity, consent, and data-classification policy to a shutdown path. Refused.

**Retry a collision or write failure.** A shutdown path must be bounded and
silent. Refused.

## Revisit conditions

Revisit before adding a Linux host panic-containment route, a timestamp, record
reader, migration, encryption, another record kind, system-service location,
cross-process coordination, general logging, or any report transmission.
