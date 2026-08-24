# Anodrel development native form template

**Status:** Implemented and covered by automated verification. This is a
Windows development template, not a product packaging or application-identity
format.

## Purpose

This template creates a small first-party Rust executable that renders one fixed
native form, waits for the person's submit action, reads the current
whole-surface field snapshot once, then closes its own session. It demonstrates
the important Anodrel rule that an application learns a value only after it
asks: it never sees keystrokes, typing timing, edit history, focus, caret, or
selection.

The template keeps bootstrap records, wire frames, request IDs, raw JSON,
field selectors, native controls, keyboard messages, host policy, package
identity, and lifecycle code out of generated application source. It is
separate from the regular UI, native-menu, and multi-window templates; none of
those routes gains form read authority because this one exists.

## Generator contract

`anodrel-native-app-tool init-form <destination> <project-slug>
<display-label>` will accept the existing validated new-directory arguments and
write only:

~~~text
my-native-form-app/
|- Cargo.toml
|- README.md
`- src/
   `- main.rs
~~~

Every Anodrel path will be relative to the local checkout. The generator will
not install, run, sign, package, register, trust, or assign identity to the
executable. It will not accept a field definition, action, document, title,
capability list, native setting, source path, secret, selector, or network
input.

The generated document will contain exactly one enabled `template.form.name`
field, initially empty, with a 96-character limit and an accessible label, plus
one enabled `template.form.submit` action. Its source will request one typed
snapshot only after that action and will not print, display, save, transmit, or
reinsert the returned value into a document.

## Typed client contract

`anodrel-windows-ui-sdk` exposes:

| Method | Input | Typed result | Protocol | Required grant |
| --- | --- | --- | --- | --- |
| `read_fields` | none | whole current `UiFieldSnapshot` | 1.15 | `ui.fields.read` |

`UiFieldSnapshot` will expose its field values only as one ordered slice. A
field value exposes only its validated element ID and current value. The
snapshot cannot select one field, report whether a person edited or focused it,
or carry a caret, selection, timestamp, key, event, or native resource.

The client will reject a response with more than 64 fields, duplicate or
out-of-order IDs, unknown fields, invalid IDs, invalid values, or any
noncanonical response shape. It will not silently drop malformed values.

## Development host session

The development-host command is:

~~~text
anodrel-windows-host --native-form-template-client <client.exe>
~~~

It will create one host-controlled Windows session and grant exactly:

- `ui.document.write`;
- `ui.events.read`;
- `ui.fields.read`; and
- `session.close`.

The host owns the native field rendering, input, current text, focus, caret,
selection, field-read UI-thread bridge, window, process, pipe worker, and
shutdown. The generated app has no keyboard input source, native handle,
control identifier, field selector, title, identity, endpoint, or native
authority.

## Verification

Automated checks cover the typed request version and exact empty payload,
closed snapshot parsing and rejection, generated-project isolation build, and a
real authenticated child session. That session proof delivers the form
document, supplies the submit action, completes the host's field-read mailbox
with the fixed whole-surface snapshot, then verifies clean session close and
child exit.

The manual Windows check builds a generated project, enters text in the visible
field, activates **Submit form**, and observes a clean close. The
generated app must not echo the entered text into the window or console. The
check will not claim a secret field, live validation, production launch, or
value persistence.

See `docs/UI_FIELDS.md`, Decision 0067, and Decision 0095.
