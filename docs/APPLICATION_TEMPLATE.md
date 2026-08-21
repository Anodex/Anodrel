# Creating a first Anodrel application package

This is the first small, runnable Anodrel project template. It creates the
current `anodrel.text.v1` package: a manifest plus one digest-verified plain
text document that the Windows host draws itself.

It is deliberately not an executable application template. It does not create
JavaScript, HTML, a browser surface, a native bridge, an installed application
record, a capability grant, a signature, or an update mechanism. Those product
boundaries do not exist yet. See `docs/APPLICATIONS.md` and Decision 0077.

## Create a package

Run from the repository root:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-package-tool -- init out/hello-anodrel org.example.hello "Hello Anodrel" "Hello from a native Anodrel package."
~~~

The destination must not already exist. On success it contains exactly:

~~~text
hello-anodrel/
|- .gitattributes
|- anodrel.application.json
`- content/
   `- main.txt
~~~

The native tool validates the supplied identity and text before it creates the
directory. It writes UTF-8 text without a byte-order mark, normalises line
endings to LF, and calculates the manifest's lowercase SHA-256 digest from the
exact resulting bytes. It then loads the package through the same owned
validator the host uses. Editing `content/main.txt` later requires regenerating
the manifest digest; rerun the tool into a new directory instead of changing
the generated manifest by hand. Its package-local `.gitattributes` keeps Git
from line-ending-converting the digest-verified content when the package is put
under version control.

On Windows, `scripts/new-application.ps1` accepts the equivalent named
arguments and delegates to this same native tool. It is a convenience wrapper,
not a second generator.

## Run the package

The host command remains explicit:

~~~powershell
$manifest = (Resolve-Path .\out\hello-anodrel\anodrel.application.json).Path
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --application $manifest
~~~

The host should open one native window headed **Hello Anodrel**, report verified
content integrity, and draw the supplied text. It independently verifies the
manifest, package containment, content limits, and digest; a successful
generator run is not a replacement for that check.

To verify the package without opening a window, run:

~~~text
cargo run --release --manifest-path native/Cargo.toml -p anodrel-package-tool -- verify out/hello-anodrel/anodrel.application.json
~~~

It prints only the validated identity, declared content path, digest, and byte
length—not the application text.

## Limits and failure behaviour

The generated package follows Application Packages v1 exactly:

- `applicationId` is 3–128 ASCII characters of lowercase letters, digits,
  `.`, `-`, or `_`, beginning and ending with a letter or digit;
- `displayName` is non-empty, has no control characters, and is at most 80 UTF-8
  bytes; and
- text is UTF-8 plain text of at most 8 KiB, 4,096 Unicode scalar values, 128
  lines, and 160 scalar values per line, with LF as its only control character.

The script refuses invalid values and an existing destination; it never merges
with or deletes a package. The generated surface has no code execution,
navigation, links, forms, permissions, or application-to-host API. For the
full boundary, see `docs/APPLICATIONS.md`.

## Verify the generator

Run the native tool's focused tests from the repository root:

~~~text
cargo test --manifest-path native/Cargo.toml -p anodrel-application -p anodrel-package-tool
~~~

They create packages only in unique temporary directories, verify the generated
bytes, Git attribute rule, digest, and host-compatible package facts, prove that
invalid input and an existing destination are refused, then remove their own
temporary directories.

The existing Windows wrapper test remains useful for its PowerShell invocation
path. Add `-VerifyHost` to run the generated package through the host's
non-visual startup checks as well:

~~~powershell
powershell -File .\scripts\test-new-application.ps1 -VerifyHost
~~~
