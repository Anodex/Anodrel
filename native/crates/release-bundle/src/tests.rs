//! Contract tests for the owned release bundle.

use crate::{BundleEntryInput, ReleaseBundle, ReleaseBundleError, encode};

#[test]
fn an_ordered_bundle_round_trips_without_copying_its_file_contents() {
    let bytes = encode(&[
        BundleEntryInput {
            path: "bin/product.exe",
            contents: b"image",
        },
        BundleEntryInput {
            path: "content/main.txt",
            contents: b"hello",
        },
    ])
    .expect("the ordered entries encode");
    let bundle = ReleaseBundle::parse(&bytes).expect("the encoded bundle parses");

    assert_eq!(bundle.entries().len(), 2);
    assert_eq!(bundle.entries()[0].path(), "bin/product.exe");
    assert_eq!(bundle.file("content/main.txt"), Some(&b"hello"[..]));
    assert_eq!(bundle.file("missing"), None);
}

#[test]
fn encoding_requires_strictly_ascending_safe_paths() {
    let unsorted = encode(&[
        BundleEntryInput {
            path: "content/main.txt",
            contents: b"first",
        },
        BundleEntryInput {
            path: "bin/product.exe",
            contents: b"second",
        },
    ]);
    assert_eq!(unsorted, Err(ReleaseBundleError::EntryOrderInvalid));

    let traversal = encode(&[BundleEntryInput {
        path: "content/../main.txt",
        contents: b"no",
    }]);
    assert_eq!(traversal, Err(ReleaseBundleError::PathInvalid));
}

#[test]
fn parsing_rejects_integrity_truncation_and_trailing_byte_failures() {
    let original = encode(&[BundleEntryInput {
        path: "content/main.txt",
        contents: b"hello",
    }])
    .expect("the entry encodes");

    let mut digest_mismatch = original.clone();
    *digest_mismatch.last_mut().expect("bundle has content") ^= 1;
    assert!(matches!(
        ReleaseBundle::parse(&digest_mismatch),
        Err(ReleaseBundleError::DigestMismatch)
    ));

    let truncated = &original[..original.len() - 1];
    assert!(matches!(
        ReleaseBundle::parse(truncated),
        Err(ReleaseBundleError::Truncated)
    ));

    let mut trailing = original;
    trailing.push(0);
    assert!(matches!(
        ReleaseBundle::parse(&trailing),
        Err(ReleaseBundleError::TrailingData)
    ));
}
