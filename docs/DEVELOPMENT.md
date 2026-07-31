# Anodrel Development Guide

## Current state

Anodrel is in the documentation and architecture phase. There is no build
toolchain or runtime implementation yet.

Do not invent commands that are not present in the repository. Once the first
language and host are selected, this guide becomes the source of truth for
installation, development, testing, packaging, and release commands.

## Working process

1. Start from an issue or a written task with a clear acceptance condition.
2. Read the root documentation and relevant decision records.
3. Keep changes within the correct repository and layer.
4. Update architecture or decision documentation when a boundary changes.
5. Add tests for protocol and security-sensitive behavior.
6. Run the documented verification commands.
7. Review the Git diff for unrelated files, secrets, generated output, and
   accidental Anodex coupling.

## Change categories

### Architecture changes

Update docs/ARCHITECTURE.md and add a numbered decision record under
docs/decisions/ when the change affects ownership, boundaries, protocol,
security, or supported platforms.

### Protocol changes

Document the schema, compatibility rule, migration behavior, and tests before
changing an existing message. Prefer additive changes over breaking changes.

### Native host changes

Document the operating-system behavior, permission requirements, failure modes,
and cleanup behavior. Include manual verification instructions.

### Application changes

Keep application-specific logic in apps/ or the consuming application. Do not
put product behavior into the platform core merely because it is convenient.

## Git hygiene

- Keep Anodrel commits separate from Anodex commits.
- Do not use the Anodex repository as a submodule or working directory shortcut
  during the foundation phase.
- Do not commit secrets, credentials, downloaded models, installers, logs, or
  generated build output.
- Use focused commits that leave the repository understandable at each step.

## Documentation standard

Documentation should answer:

- What does this component own?
- What does it deliberately not own?
- Who calls it?
- What can fail?
- What security assumptions does it make?
- How is it tested?
- How can it be replaced later?
