# Anodrel command-line example

This is a small command-line application that uses only `@anodrel/sdk` and the
development `@anodrel/mock-host`. It asks for the public host health and the
capabilities already issued to its own session, then prints those facts as
JSON.

It has no native imports, no direct Windows call, no browser runtime, and no
way to choose its identity or grant itself a capability. The mock is for local
development and contract tests; this example is not an installed command or a
packaged application process.

Run it from the repository root:

~~~text
npm run cli-demo
~~~

See `docs/SDK.md` for the SDK boundary and `docs/PROTOCOL.md` for the result
shapes and capability rules.
