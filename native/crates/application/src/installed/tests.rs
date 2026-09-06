//! Shared fixtures for installed-application verification.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::{InstalledApplication, InstalledApplicationError};
use crate::sha256;

const APPLICATION_ID: &str = "org.anodrel.sample";
const FIXTURE_REMOVE_ATTEMPTS: u32 = 8;
const CONCURRENT_FIXTURE_COUNT: usize = 16;
static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    policy_root: PathBuf,
    package_root: PathBuf,
    record_path: PathBuf,
    executable_path: PathBuf,
}

impl Fixture {
    fn remove(self) {
        for attempt in 1..=FIXTURE_REMOVE_ATTEMPTS {
            match fs::remove_dir_all(&self.root) {
                Ok(()) => return,
                // Windows can briefly retain a just-read test file for a
                // scanner. A bounded retry keeps that external race from
                // making the suite flaky without masking a persistent leak.
                Err(error)
                    if error.kind() == std::io::ErrorKind::PermissionDenied
                        && attempt < FIXTURE_REMOVE_ATTEMPTS =>
                {
                    thread::sleep(Duration::from_millis(u64::from(attempt) * 10));
                }
                Err(error) => panic!("fixture directory is removed: {error}"),
            }
        }
        unreachable!("fixture cleanup either succeeds or reports its final error");
    }
}

fn fixture() -> Fixture {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "anodrel-installed-application-{}-{unique}-{sequence}",
        std::process::id(),
    ));
    let policy_root = root.join("policy");
    let package_root = root.join("package");
    let content_path = package_root.join("content").join("main.txt");
    let executable_path = package_root.join("bin").join("sample.exe");
    fs::create_dir(&root).expect("fixture root is newly created");
    fs::create_dir_all(content_path.parent().expect("content has parent"))
        .expect("content directory is created");
    fs::create_dir_all(executable_path.parent().expect("executable has parent"))
        .expect("executable directory is created");
    fs::create_dir_all(&policy_root).expect("policy directory is created");

    let content = b"Hello from the installed package.\n";
    let executable = b"Anodrel fixture executable";
    fs::write(&content_path, content).expect("content is written");
    fs::write(&executable_path, executable).expect("executable is written");
    fs::write(
        package_root.join("anodrel.application.json"),
        format!(
            r#"{{
                "manifestVersion": {{"major": 1, "minor": 0}},
                "applicationId": "{APPLICATION_ID}",
                "displayName": "Anodrel Sample",
                "content": {{
                    "format": "anodrel.text.v1",
                    "path": "content/main.txt",
                    "sha256": "{}"
                }}
            }}"#,
            sha256::to_lower_hex(&sha256::digest(content)),
        ),
    )
    .expect("package manifest is written");

    let record_path = policy_root.join("sample.json");
    write_record(
        &record_path,
        APPLICATION_ID,
        &package_root,
        "bin/sample.exe",
        &sha256::to_lower_hex(&sha256::digest(executable)),
    );

    Fixture {
        root,
        policy_root,
        package_root,
        record_path,
        executable_path,
    }
}

#[test]
fn concurrent_fixtures_have_distinct_owned_roots() {
    let fixtures = thread::scope(|scope| {
        (0..CONCURRENT_FIXTURE_COUNT)
            .map(|_| scope.spawn(fixture))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|worker| worker.join().expect("fixture worker completes"))
            .collect::<Vec<_>>()
    });
    let roots = fixtures
        .iter()
        .map(|fixture| fixture.root.clone())
        .collect::<BTreeSet<_>>();

    assert_eq!(roots.len(), CONCURRENT_FIXTURE_COUNT);
    for fixture in fixtures {
        assert!(fixture.root.is_dir());
        fixture.remove();
    }
}

fn write_record(
    path: &Path,
    application_id: &str,
    package_root: &Path,
    executable_path: &str,
    executable_digest: &str,
) {
    let package_root = package_root
        .to_str()
        .expect("temporary path is valid Unicode")
        .replace('\\', "\\\\");
    fs::write(
        path,
        format!(
            r#"{{
                "recordVersion": {{"major": 1, "minor": 0}},
                "applicationId": "{application_id}",
                "packageRoot": "{package_root}",
                "executable": {{
                    "path": "{executable_path}",
                    "sha256": "{executable_digest}"
                }},
                "publisher": {{
                    "leafCertificateSha256": "{}"
                }}
            }}"#,
            sha256::to_lower_hex(&[0xA5; 32]),
        ),
    )
    .expect("installed record is written");
}

mod grants;
mod legacy;
mod validation;
