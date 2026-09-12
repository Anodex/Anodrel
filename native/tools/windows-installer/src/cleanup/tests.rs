//! Real Windows executable lifetime tests, isolated from machine policy and trust.

use super::*;
use crate::{recovery::raw::remove_normal_tree, test_support::TestDirectory};
use std::{fs, os::windows::fs::OpenOptionsExt};

#[test]
fn unsigned_images_cannot_enter_any_cleanup_route() {
    assert_eq!(
        begin_current_uninstall_cleanup(),
        Err(CleanupError::Verification)
    );
    assert_eq!(
        run_current_uninstall_cleanup(),
        Err(CleanupError::Verification)
    );
    assert_eq!(
        retire_current_cleanup_cache(),
        Err(CleanupError::Verification)
    );
}

#[test]
fn a_running_image_blocks_deletion_but_exit_allows_same_version_reinstall() {
    let dir = TestDirectory::new("cleanup-process");
    let package = dir.path().join("0.1.0");
    fs::create_dir(&package).unwrap();
    let image = package.join("uninstaller.exe");
    fs::copy(std::env::current_exe().unwrap(), &image).unwrap();
    let ready = dir.path().join("ready");
    let mut child = PendingChild(Some(
        Command::new(&image)
            .args(["--ignored", "--exact", "cleanup::tests::mapped_image_child"])
            .env("ANODREL_CLEANUP_TEST_READY", &ready)
            .current_dir(dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .unwrap(),
    ));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        assert!(
            Instant::now() < deadline,
            "child did not reach its bounded wait"
        );
        assert!(child.0.as_mut().unwrap().try_wait().unwrap().is_none());
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        remove_normal_tree(&package).is_err(),
        "Windows must retain a mapped image"
    );
    child
        .0
        .as_mut()
        .unwrap()
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&COMMIT)
        .unwrap();
    loop {
        if let Some(status) = child.0.as_mut().unwrap().try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline, "child failed to exit");
        thread::sleep(Duration::from_millis(10));
    }
    remove_normal_tree(&package).unwrap();
    assert!(!package.exists());
    // No reboot API or machine policy is involved; the same version is reusable now.
    fs::create_dir(&package).unwrap();
    fs::write(package.join("replacement"), b"new installation").unwrap();
}

#[test]
#[ignore = "bounded child process launched by the lifecycle test"]
fn mapped_image_child() {
    let ready = std::env::var_os("ANODREL_CLEANUP_TEST_READY").expect("test-only child path");
    fs::write(ready, b"ready").unwrap();
    channel::read_stdin(COMMIT, Duration::from_secs(8)).unwrap();
}

#[test]
fn a_busy_file_is_retained_and_cleanup_can_be_retried_after_release() {
    let dir = TestDirectory::new("cleanup-busy-file");
    let package = dir.path().join("0.1.0");
    fs::create_dir(&package).unwrap();
    let file = package.join("busy");
    fs::write(&file, b"retained").unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    assert!(remove_normal_tree(&package).is_err());
    assert!(file.exists());
    drop(held);
    remove_normal_tree(&package).unwrap();
    assert!(!package.exists());
}

#[test]
fn commit_evidence_is_empty_and_cannot_be_overwritten() {
    let dir = TestDirectory::new("cleanup-marker");
    let marker = cache::mark_committed(dir.path()).unwrap();
    assert_eq!(fs::metadata(marker).unwrap().len(), 0);
    assert_eq!(cache::mark_committed(dir.path()), Err(CleanupError::Cache));
}

#[test]
fn retirement_preserves_packages_unknown_contents_and_unverified_images() {
    let dir = TestDirectory::new("cleanup-retirement");
    let package = dir.path().join("0.1.0");
    fs::create_dir(&package).unwrap();
    fs::write(package.join("preserve"), b"original").unwrap();
    let empty = dir.path().join(".anodrel-cleanup-1-1");
    fs::create_dir(&empty).unwrap();
    cache::retire(dir.path(), &manifest()).unwrap();
    assert!(!empty.exists());
    let stage = dir.path().join(".anodrel-cleanup-1-2");
    fs::create_dir(&stage).unwrap();
    let unrelated = stage.join("unknown");
    fs::write(&unrelated, b"preserve").unwrap();
    assert_eq!(
        cache::retire(dir.path(), &manifest()),
        Err(CleanupError::Cache)
    );
    assert!(unrelated.exists());
    fs::remove_file(&unrelated).unwrap();
    let image = stage.join(stage::IMAGE);
    fs::copy(std::env::current_exe().unwrap(), &image).unwrap();
    cache::mark_committed(&stage).unwrap();
    assert_eq!(
        cache::retire(dir.path(), &manifest()),
        Err(CleanupError::Verification)
    );
    assert!(image.exists());
    assert_eq!(fs::read(package.join("preserve")).unwrap(), b"original");
}

fn manifest() -> crate::ReleaseManifest {
    crate::ReleaseManifest::parse(r#"{
        "formatVersion":{"major":1,"minor":0},
        "applicationId":"org.anodrel.cleanup-test",
        "packageVersion":{"major":0,"minor":1,"patch":0},
        "executable":{"path":"bin/Product.exe","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"},
        "publisher":{"leafCertificateSha256":"7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"},
        "capabilities":[],"networkOrigins":[],
        "payload":{"byteLength":1,"sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}
    }"#).unwrap()
}
