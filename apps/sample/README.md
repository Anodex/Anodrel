# Anodrel sample application

This sample is a deliberately small SDK consumer. It does not import native
host internals; it receives a transport backed by the mock host and uses the
public client API.

Run it from the repository root with:

~~~text
npm run demo
~~~

`src/native-client.ts` is a separate development-only entry point. The direct
Windows host launches it with a private standard-input invitation so it can
authenticate to the real named pipe and call `platform.health`. Its command is
documented in `docs/DEVELOPMENT.md`; it is not a packaged or trusted content
host.

`anodrel.application.json` and `content/main.txt` are a separate static
application package for the first Windows content surface. The host
verifies the declared SHA-256 digest and package containment before drawing the
text. To view it, run:

~~~text
cargo run --manifest-path native/Cargo.toml -p anodrel-windows-host -- --application apps/sample/anodrel.application.json
~~~

If `content/main.txt` changes, its SHA-256 value in the manifest must be
updated in the same review. Its Git attribute preserves exact bytes so line
ending conversion cannot invalidate that digest. This format has no scripts,
executable entry point, navigation, or native bridge.
