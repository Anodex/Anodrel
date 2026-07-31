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
