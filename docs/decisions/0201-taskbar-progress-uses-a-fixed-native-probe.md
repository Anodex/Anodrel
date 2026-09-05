# Decision 0201: Taskbar progress uses a fixed native probe

**Status:** Accepted

**Date:** 2026-09-04

## Context

The product-update progress adapter is direct Windows COM. Unit checks can
prove its bounded inputs and vtable layout, but they cannot prove that Explorer
delivered `TaskbarButtonCreated` or rendered an indicator for an actual host
window. The signed product fixture is deliberately unavailable until a person
chooses to change development-machine certificate trust.

## Decision

Provide one no-argument `--taskbar-progress-probe` host diagnostic. It opens a
fixed host document window, waits for that exact window's
`TaskbarButtonCreated` message, presents fixed activity and 0–100% determinate
taskbar states, clears the state, then closes itself with a fixed success or
safe failure. It takes no product identity, application data, endpoint,
catalogue, file, installer, certificate, command, or timing input.

The probe owns its own short-lived UI-thread state and never reuses a product
controller, update worker, application protocol, or taskbar object. A Shell
restart resets only its readiness state; it waits for a fresh button-created
message before another direct taskbar call.

## Consequences

- The direct COM and Windows message ordering can be manually checked without
  modifying machine trust, policy, packages, cache, or an installed product.
- The probe is evidence for the taskbar adapter only. It does not prove signed
  update discovery, consent, private transfer, UAC, policy proof, or product
  caption integration.
- A missing taskbar service or readiness signal ends the probe safely rather
  than assuming an indicator was visible.

## Revisit conditions

Revisit for automated desktop capture, an external UI-testing harness, another
operating-system taskbar, persistent diagnostics, a product-facing taskbar
setting, or signed end-to-end release acceptance.
