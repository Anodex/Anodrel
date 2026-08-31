# Release bundle format

**Status:** Owned encoder and borrowed decoder implemented. No installer
extraction path is implemented yet.

## Purpose

`anodrel.bundle.v1` is the exact raw payload carried beside an
`anodrel.release.v1` manifest in a signed installer image. It is an Anodrel
binary format, not a ZIP, CAB, MSI, or general archive.

The release manifest authenticates the complete bundle byte sequence. This
format also authenticates each file before a later installer writes it to a
private staging directory.

## Wire layout

All integers are unsigned little-endian. The payload has at most **512 MiB**,
contains at most **128** files, and has no compression or trailing data.

~~~text
offset  size  meaning
0       4     ASCII `ANDB`
4       1     format major: 1
5       1     format minor: 0
6       2     entry count
8       ...   repeated entry records in strictly ascending UTF-8 byte-path order

entry record:
0       2     UTF-8 path byte length (1 through 240)
2       4     content byte length
6       32    SHA-256 of content bytes
38      path length   relative path bytes
...     content length raw content bytes
~~~

Each content range must remain inside the payload. The final entry must end
exactly at the payload end.

## Path rules

An entry path names one regular file below the future installer-selected root.
It uses forward slashes and has no leading slash, backslash, drive colon, empty
component, `.` component, `..` component, or control character. Directory
entries, links, alternate data streams, permissions, timestamps, and special
files do not exist in version 1.

Paths must be strictly ascending by their raw UTF-8 bytes. This makes the format
deterministic and rejects duplicate or ambiguous names without filesystem I/O.

## Decoder boundary

The owned decoder receives borrowed bytes and produces file metadata plus
borrowed content slices. It verifies every declared SHA-256 digest during parse.
It does not open a file, allocate a copy of file contents, write a path, create
a directory, change machine policy, evaluate a signature, or launch a process.

`anodrel-release-bundle` implements this encoder and decoder. The Windows
installer foundation first checks the manifest's total payload length and digest,
then calls the decoder. A corrupt or substituted payload therefore cannot reach
file-level parsing merely because its manifest has a valid shape.

The installer reads a release payload only from its own fixed `RT_RCDATA`
resource (`0xA142`); the companion signed manifest is resource `0xA141`.

A later Windows extraction module must recheck each path while creating only
new files below its private staging root, then validate the staged package and
record before registry publication.

## Compatibility

Version 1.0 accepts exactly this header and entry layout. A newer minor version
must preserve every version-1.0 field and add only documented trailing or
explicitly versioned data. A newer major version is rejected by the version-1
decoder.

See [Windows installer contract](WINDOWS_INSTALLER.md) and Decisions 0140 and
0141.
