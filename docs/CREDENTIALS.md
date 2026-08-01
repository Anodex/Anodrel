# Secure credentials v1

**Status:** Windows host foundation. This is a host-only credential-store
contract, not a public protocol operation or application permission.

## Purpose and boundary

Applications eventually need small durable secrets such as refresh tokens,
device keys, and service credentials. They must not choose an arbitrary Windows
credential target, read another application's secret, persist a secret in the
application package, or send one to a renderer or diagnostic surface.

`anodrel-credentials` owns validated names, opaque in-memory secret values,
and the fixed target namespace. `anodrel-windows-credentials` calls only the
current user's Windows Credential Manager generic-credential APIs:
`CredWriteW`, `CredReadW`, `CredDeleteW`, and `CredFree`.

The first adapter exposes no credential enumeration, search, sharing,
prompting, export, user name, attribute, roaming, service-account, or public
protocol interface. It does not create directories or use Anodrel's application
path layout.

## Namespace and inputs

A host supplies a validated `ApplicationIdentity`, a `CredentialName`, and,
for writing, an opaque `Secret`. Windows target names are derived exactly as:

~~~text
Anodrel/v1/<applicationId>/<credentialName>
~~~

`applicationId` uses the established application-ID grammar. A credential name
is 1 to 64 ASCII bytes using only lowercase letters, digits, `.`, `-`, or `_`,
and must start and end with a lowercase letter or digit. The fixed namespace
prevents a caller from supplying separators, a Windows credential prefix, or
another application's target.

Secrets are non-empty opaque byte sequences up to **2,048 bytes**. This is
below Windows Credential Manager's generic credential-blob limit. Secret and
target debug output is redacted; errors never include a target, application ID,
secret, raw Windows status, or current-user information.

The portable secret type can convert arbitrary bytes to and from exact
lowercase hexadecimal for a future explicitly authenticated protocol boundary.
The encoding is canonical (two lowercase characters per byte), bounded to
4,096 characters, and accepts no whitespace or alternate spelling. It does not
itself create a public credential operation or relax the secret-handling rules.

## Windows behavior

The adapter always uses `CRED_TYPE_GENERIC` and `CRED_PERSIST_LOCAL_MACHINE`.
Windows associates the credential set with the current process token; this is
current-user state on the local machine, not a machine-wide or cross-user
store. Writing replaces only an existing credential of the same derived target
and generic type. Reading returns `NotFound` when that exact target is absent.
Deletion is idempotent: it returns `false` for an absent credential and never
enumerates the store.

The adapter copies a successful Windows-owned credential blob into the opaque
secret type, clears the returned buffer before `CredFree`, and clears its own
secret allocation when the value is dropped. It never writes secret bytes to
logs, events, errors, output, paths, or command-line data.

Credential Manager calls are synchronous and must run off a future host UI
thread. The adapter has no global cache or background work; each request makes
one operating-system call.

## Compatibility and public exposure

This is credential layout version 1. The `Anodrel/v1` target prefix, type,
persist policy, and input limits are stable. Changing any of them requires a
migration plan, a documented compatibility version, and tests that can compare
the old and new stores.

No `platform.credentials.*` operation exists in Protocol v1. A future public
surface must define read, write, and delete capabilities; authenticated session
binding; consent and revocation behavior; cancellation; safe error mapping;
and mock/native compatibility tests before it can provide a secret to an
application. Until then, only trusted native host code can use this adapter.

## Verification

Unit tests cover name and target validation, opaque secret diagnostics, bounds,
and safe error mapping. The Windows integration test writes one uniquely named
test credential in the Anodrel namespace, reads it back, and removes it in the
same test (including cleanup on an assertion failure). It does not print the
target or secret.

~~~text
cargo test --manifest-path native/Cargo.toml -p anodrel-credentials
cargo test --manifest-path native/Cargo.toml -p anodrel-windows-credentials
~~~

Decision 0022 records the security and persistence choice.
