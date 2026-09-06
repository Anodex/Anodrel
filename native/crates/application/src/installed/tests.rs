//! Shared fixtures for installed-application verification.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    thread,
};

use super::{InstalledApplication, InstalledApplicationError};
use crate::{sha256, test_support::TestDirectory};

const APPLICATION_ID: &str = "org.anodrel.sample";
const CONCURRENT_FIXTURE_COUNT: usize = 16;

struct Fixture {
    root: TestDirectory,
    policy_root: PathBuf,
    package_root: PathBuf,
    record_path: PathBuf,
    executable_path: PathBuf,
}

impl Fixture {
    fn remove(self) {
        self.root.remove();
    }
}

fn fixture() -> Fixture {
    let root = TestDirectory::new("installed-application");
    let policy_root = root.path().join("policy");
    let package_root = root.path().join("package");
    let content_path = package_root.join("content").join("main.txt");
    let executable_path = package_root.join("bin").join("sample.exe");
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
        .map(|fixture| fixture.root.path().to_path_buf())
        .collect::<BTreeSet<_>>();

    assert_eq!(roots.len(), CONCURRENT_FIXTURE_COUNT);
    for fixture in fixtures {
        assert!(fixture.root.path().is_dir());
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
