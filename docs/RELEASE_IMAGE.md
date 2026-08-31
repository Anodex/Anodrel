# Windows release image assembly

**Status:** Contract for the owned pre-signing image builder. It creates no
installed application and no machine policy.

## Purpose

`anodrel-release-image` creates a new resource-bearing copy of an unsigned
Windows installer template. It uses only Anodrel code and direct Kernel32
resource-update APIs; it does not use a resource compiler, installer framework,
or archive tool.

## Inputs and output

~~~text
unsigned installer template.exe
strict anodrel.release.v1 manifest
checked anodrel.bundle.v1 payload
             |
             v
new unsigned resource-bearing installer.exe
             |
             v
separate production signing step
~~~

The build operator supplies the inputs. The output path must be absolute and
previously absent. The tool refuses to overwrite a file or modify the template
in place.

## Current implementation

`native/tools/release-image` implements the `embed` command. Its direct Windows
integration test copies the real test PE to a fresh temporary path, writes a
checked manifest and bundle through the resource-update transaction, reloads the
result as data-only, and compares both stored resource byte sequences. A second
test proves an existing output is left untouched.

The resulting image is still unsigned. It cannot activate the installer until
the separate signing step produces a Windows-accepted image with the same
embedded publisher identity.

## Assembly sequence

1. Read and strictly validate the manifest and payload as one release chain.
2. Copy the template to the new output path.
3. Open only that copied, non-running PE file with `BeginUpdateResourceW`.
4. Add or replace exactly two neutral-language `RT_RCDATA` resources:

   | Identifier | Content |
   | --- | --- |
   | `0xA141` | manifest UTF-8 bytes |
   | `0xA142` | bundle bytes |

5. Commit both updates together with `EndUpdateResourceW`.
6. Reload the output as data-only PE content and require each resource to equal
   its original input bytes.
7. Report that the verified output is **unsigned**. The separate signing step
   must happen before distribution or installation.

If an update fails before commit, the resource update is discarded. The copied
output remains an ordinary unsigned template copy, never a claimed release.

## Boundaries

This tool does not write inside an installed Anodrel directory, update a running
executable, install a registry record, create trust, choose a certificate,
download data, or run the result. Resource modification invalidates an existing
signature, so signed input templates are not a release shortcut.

See [Windows installer contract](WINDOWS_INSTALLER.md), [release bundle](RELEASE_BUNDLE.md),
and Decisions 0140–0144.
