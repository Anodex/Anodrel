# Decision 0132: Linux Wayland Lab teardown is ordered and bounded

- Status: Accepted
- Date: 2026-08-30

## Context

The Linux development-window Lab now owns one fixed Wayland view alongside a
private child session. Dropping the Wayland socket eventually lets a compositor
discard the client, but it leaves the host unable to state or test the order in
which it releases the objects it created. Waiting for a compositor round-trip
while the child is already ending would make shutdown depend on a desktop that
may be suspended, gone, or refusing the connection.

The direct adapter binds `wl_seat` and `wl_pointer` at version 1. Their explicit
release requests were added only in later protocol versions, so sending those
requests would be invalid on a compositor that correctly negotiated version 1.

## Decision

`LinuxWaylandLab::close` is an idempotent, host-only best-effort teardown. It
sends only the supported fixed destructor requests, in dependency order:

1. `xdg_toplevel`, `xdg_surface`, and `wl_surface`;
2. each fixed `wl_buffer`;
3. `xdg_wm_base`, `wl_shm`, `wl_compositor`, and `wl_registry`.

It first marks the Lab closed, so later presentation or event waiting returns a
closed desktop category rather than sending a new request. It does not wait for
a callback, frame release, acknowledgement, or server response. A failed send
is ignored because teardown already has the fixed final fallback: the owned
socket closes immediately after the Lab drops.

The version-1 `wl_seat` and `wl_pointer` stay attached to that final socket
close. No later-version release opcode is guessed, and no raw object identifier
or close result becomes application-visible. `Drop` follows the same path, and
the development-window session explicitly closes its view before it stops and
joins its child session.

## Consequences

- The host has a short, testable plan for every fixed Wayland object it owns.
- Host-initiated child/session shutdown no longer relies only on connection
  loss to begin destroying visible desktop roles.
- Teardown adds no allocation, worker, round trip, sleep, timer, or
  compositor-paced wait to the shutdown path.
- A compositor close and a transport/child close still converge safely: an
  already broken stream cannot turn shutdown into a second public failure.

## Deliberately absent

- an application-controlled close request, callback, acknowledgement, timeout,
  object identifier, connection API, or native handle;
- waits for buffer release, explicit release of unnegotiated seat/pointer
  protocol versions, renderer cleanup callbacks, or background cleanup work;
- application document composition, Linux input delivery, product policy,
  package identity, installation, updates, or accessibility.

## Alternatives considered

**Rely on socket drop alone.** Correct enough for eventual server cleanup but
not explicit or testable for the child/view ownership boundary. Refused.

**Synchronously round-trip after every destroy request.** This makes process
shutdown wait for an untrusted desktop server and can strand the child owner.
Refused.

**Bind newer seat and pointer versions only to obtain release.** That broadens
the accepted event surface for a cleanup convenience. Refused.

## Revisit conditions

Revisit before adding a different negotiated Wayland version, reusable object
lifetimes, application-controlled close behavior, multi-window support,
asynchronous frame pacing, or a Linux product host.
