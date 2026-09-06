//! Installed-record identity, digest, and containment validation checks.

use super::*;

#[test]
fn rejects_a_trusted_record_for_a_different_policy_key_identity() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path).expect("record is read");

    assert!(matches!(
        InstalledApplication::load_from_trusted_record(&record, "org.anodrel.other"),
        Err(InstalledApplicationError::ApplicationIdentityMismatch)
    ));
    fixture.remove();
}

#[test]
fn rejects_a_record_that_disagrees_with_the_package_identity() {
    let fixture = fixture();
    let contents = fs::read_to_string(&fixture.record_path).expect("record is read");
    fs::write(
        &fixture.record_path,
        contents.replacen(APPLICATION_ID, "org.anodrel.other", 1),
    )
    .expect("record is changed");

    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::ApplicationIdentityMismatch)
    ));
    fixture.remove();
}

#[test]
fn rejects_a_record_outside_the_selected_policy_root() {
    let fixture = fixture();
    let other_policy_root = fixture.root.path().join("other-policy");
    fs::create_dir(&other_policy_root).expect("other policy directory is created");

    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &other_policy_root),
        Err(InstalledApplicationError::RecordOutsidePolicyRoot)
    ));
    fixture.remove();
}

#[test]
fn rejects_an_executable_that_changes_after_the_record_is_written() {
    let fixture = fixture();
    fs::write(&fixture.executable_path, b"changed executable").expect("executable is changed");

    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::ExecutableDigestMismatch)
    ));
    fixture.remove();
}

#[test]
fn rejects_a_record_inside_its_mutable_package() {
    let fixture = fixture();
    let inside_package = fixture.package_root.join("installed.json");
    fs::copy(&fixture.record_path, &inside_package).expect("record is copied into package");

    assert!(matches!(
        InstalledApplication::load(&inside_package, &fixture.package_root),
        Err(InstalledApplicationError::RecordInsidePackage)
    ));
    fixture.remove();
}

#[test]
fn rejects_a_path_that_escapes_the_package_before_reading_it() {
    let fixture = fixture();
    let contents = fs::read_to_string(&fixture.record_path).expect("record is read");
    fs::write(
        &fixture.record_path,
        contents.replace("bin/sample.exe", "../sample.exe"),
    )
    .expect("record is changed");

    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidExecutablePath)
    ));
    fixture.remove();
}
