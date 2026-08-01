# Anodrel UI document interchange v1

**Status:** Foundation contract. This format can be decoded into the portable
`anodrel-ui` model. The Windows UI Lab decodes one compiled-in host fixture to
exercise the contract, and the separate explicit Windows developer preview can
render one bounded operator-selected file. The authenticated
`ui.document.replace` protocol operation can accept one more tightly bounded
document for its own session state; it does not attach a document to a window
or deliver action events yet.

## Purpose and boundary

`anodrel.ui.document.v1` is the first stable external representation of a
bounded Anodrel UI tree. It lets a future SDK or package describe the supported
version 1 subset of the portable document that a host-owned caller can
currently construct in Rust.
It is data only: it is not executable code, HTML, CSS, a web page, a renderer,
a window request, or a capability declaration.

Decoding validates the complete document before returning it. A successfully
decoded action still carries only its element ID; rendering or invoking it
cannot open a process, read a file, send a protocol message, or grant a
capability. The first authenticated replacement operation is documented in
`docs/PROTOCOL.md`. It has a `ui.document.write` capability, a 24 KiB embedded
document limit, atomic revision result, and no window or action-event bridge.
Window lifecycle, event delivery, queues, and rendering remain separate
contracts.

## Envelope

The input is one UTF-8 JSON object no larger than **64 KiB**. Duplicate fields,
unknown fields, trailing bytes, malformed Unicode, and JSON nesting beyond 64
levels are rejected. Version 1 requires exactly these top-level fields:

| Field | Type | Required value |
| --- | --- | --- |
| `format` | string | `anodrel.ui.document.v1` |
| `root` | node object | The one document root. |

The decoded tree is also subject to every `anodrel-ui` bound: at most 512 nodes,
depth 32, 32 KiB combined text and labels, valid unique element IDs, font sizes
from 8 through 96, and spacing no larger than 256 logical pixels. The codec
does not recover from invalid input or apply defaults.

## Nodes

Every node object requires `id` and `kind`. `id` follows the element-ID grammar
in `docs/UI.md`; `kind` selects one of these exact objects. All listed fields
are required and no node accepts an extra field.

### Stack

| Field | Type | Values |
| --- | --- | --- |
| `id` | string | Valid, document-unique element ID. |
| `kind` | string | `stack` |
| `axis` | string | `vertical` or `horizontal` |
| `padding` | object | Exact four-edge padding object below. |
| `gap` | integer | 0 through 256 |
| `surfaceTone` | string | `plain` or `raised` |
| `children` | array | Zero or more node objects in source order. |

`padding` requires exactly `left`, `top`, `right`, and `bottom`, each an integer
from 0 through 256.

### Text

| Field | Type | Values |
| --- | --- | --- |
| `id` | string | Valid, document-unique element ID. |
| `kind` | string | `text` |
| `value` | string | Valid bounded single-line text. |
| `fontSize` | integer | 8 through 96 |
| `tone` | string | `primary`, `secondary`, or `accent` |

### Action

| Field | Type | Values |
| --- | --- | --- |
| `id` | string | Valid, document-unique element ID. |
| `kind` | string | `action` |
| `label` | string | Valid bounded single-line text. |
| `fontSize` | integer | 8 through 96 |
| `enabled` | boolean | Whether the action participates in hit tests and focus. |
| `tone` | string | `neutral` or `accent` |

## Example

~~~json
{
  "format": "anodrel.ui.document.v1",
  "root": {
    "id": "welcome.root",
    "kind": "stack",
    "axis": "vertical",
    "padding": { "left": 24, "top": 24, "right": 24, "bottom": 24 },
    "gap": 12,
    "surfaceTone": "plain",
    "children": [
      {
        "id": "welcome.title",
        "kind": "text",
        "value": "Welcome to Anodrel",
        "fontSize": 28,
        "tone": "primary"
      },
      {
        "id": "welcome.continue",
        "kind": "action",
        "label": "Continue",
        "fontSize": 16,
        "enabled": true,
        "tone": "accent"
      }
    ]
  }
}
~~~

## Compatibility and failure behavior

Version 1 is exact. A decoder rejects every unknown field, missing field,
unknown enum value, non-integer number, out-of-range value, malformed document,
or model validation failure. In particular, the in-memory `Scroll` node has no
version 1 node form: a `kind` of `scroll` is an unsupported node kind, and the
version 1 encoder rejects a model containing one. It returns only a stable
failure category and must not echo raw untrusted input into a diagnostic.

An additive or semantic extension requires a new documented format version and
compatibility tests. A v1 decoder must not guess at an unfamiliar document. The
encoder produces deterministic JSON from a validated document only when that
representation fits inside the same 64 KiB limit; otherwise it returns the
encoded-limit failure. Consumers must treat object-member ordering as
non-semantic.

## Verification

`anodrel-ui-document` tests a known document round trip, every version 1 node kind and
appearance roles, unknown and missing fields, malformed values, unsupported
format identifiers, size limits, and model-level document limits. It has only
first-party `anodrel-ui` and `anodrel-json` dependencies and no operating-
system calls. The Windows host additionally builds and renders its compiled-in
UI Lab fixture through this decoder, offers a separate bounded developer
preview input, and uses it from the capability-checked document-replacement
state in an authenticated transport session. None of those paths attaches
application data to a native application window or delivers UI events.
