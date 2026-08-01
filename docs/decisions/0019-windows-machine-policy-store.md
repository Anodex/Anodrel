# Decision 0019: Read installed application records from machine-wide Windows registry policy

**Status:** Accepted

**Date:** 2026-07-31

## Context

Decision 0018 defines an installed application record but deliberately leaves
the trusted source of that record unresolved. Allowing a package directory,
current-user configuration, environment variable, application request, or
developer-selected file to supply publisher policy would let mutable
application-controlled data choose process authority.

The first Windows host needs a source that is directly available through the
operating system, requires no third-party runtime, and clearly separates normal
application execution from policy modification.

## Decision

Read records only from the 64-bit registry location:

~~~text
HKEY_LOCAL_MACHINE\Software\Anodrel\Applications\<applicationId>
    record    REG_SZ
~~~

`anodrel-windows-policy` opens one application key with `KEY_QUERY_VALUE` and
the 64-bit view, reads one bounded UTF-16 `REG_SZ` value, and closes the key.
It rejects missing keys, denied access, other registry value types, malformed
or oversized strings, embedded NULs, and value-size races. It has no write,
delete, enumeration, current-user, or fallback behavior.

The adapter passes the record text and the host-selected application ID to the
portable installed-record validator. The validator requires that ID to equal
both the registry record and the validated package manifest before it returns
the contained executable and comparison policy.

This establishes a read-only trust source, not installation or launch. A later
privileged installer must provision the key and define its access-control audit.
A later launch service must revalidate the executable, compare Authenticode,
create and track the process, and use bootstrap only after the full policy
check passes.

## Consequences

Positive:

- policy does not originate in an unpackaged application directory;
- the host reads the same policy location in 32-bit and 64-bit builds;
- read access is minimal and the adapter has no mutation surface;
- the portable validator remains usable by later macOS and Linux policy
  adapters.

Tradeoffs:

- records cannot be provisioned by an ordinary application;
- policy changes need a future installation workflow;
- a machine administrator can change machine policy, which is within the
  Windows administrator trust boundary.

## Revisit conditions

Revisit when Anodrel adopts a signed installer, an operating-system package
identity, enterprise policy management, a user-scoped developer mode with its
own isolated trust model, or a second operating system policy adapter.
