# Anodrel Compass

Compass is the second independent desktop package shipped with Anodrel. It is
an intentionally small `anodrel.text.v1` application: the Windows host
validates its identity, package containment, and content digest before it
draws the text in a native window.

It is not an executable application, an SDK example, a web page, or an Anodex
integration. It has no scripts, browser runtime, native bridge, permissions,
or capability grants. Its value is proving that a second application identity
can use the owned package boundary without borrowing the startup sample's
contents or importing host internals.

## Run

From the repository root:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --application apps/compass/anodrel.application.json
~~~

To validate the package without opening a window:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-package-tool -- verify apps/compass/anodrel.application.json
~~~

The content is digest-verified. Do not edit `content/main.txt` without
regenerating the manifest through the documented package tool; see
[`docs/APPLICATION_TEMPLATE.md`](../../docs/APPLICATION_TEMPLATE.md).
