//! Direct Windows Shell Link checks using a temporary ordinary directory.

use super::{ProductLaunchArguments, remove_regular_link, replace_link};
use crate::test_support::TestDirectory;

#[test]
fn writes_one_shell_link_in_a_regular_temporary_directory() {
    let directory = TestDirectory::new("shortcut");
    let executable = std::env::current_exe().expect("current test image is available");
    let link = directory.path().join("Anodrel Test.lnk");
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test")
        .expect("fixed test identity is valid");
    replace_link(
        &executable,
        executable.parent().expect("test image has a parent"),
        &arguments,
        &link,
    )
    .expect("direct Shell Link persistence succeeds");
    assert!(link.is_file());
}

#[test]
fn writes_one_shell_link_from_canonical_windows_paths() {
    let directory = TestDirectory::new("shortcut-canonical");
    let executable = std::env::current_exe().expect("current test image is available");
    let executable = std::fs::canonicalize(executable).expect("test image canonicalizes");
    let parent = executable.parent().expect("test image has a parent");
    let link = directory.path().join("Anodrel Canonical Test.lnk");
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test")
        .expect("fixed test identity is valid");

    replace_link(&executable, parent, &arguments, &link)
        .expect("canonical Windows paths persist a Shell Link");
    assert!(link.is_file());
}

#[test]
fn removes_only_the_regular_shell_link_it_just_created() {
    let directory = TestDirectory::new("shortcut");
    let executable = std::env::current_exe().expect("current test image is available");
    let link = directory.path().join("Anodrel Test.lnk");
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test")
        .expect("fixed test identity is valid");
    replace_link(
        &executable,
        executable.parent().expect("test image has a parent"),
        &arguments,
        &link,
    )
    .expect("direct Shell Link persistence succeeds");
    remove_regular_link(&link).expect("regular temporary Shell Link removes");
    assert!(!link.exists());
}

#[test]
fn persists_only_the_fixed_product_launch_arguments() {
    let directory = TestDirectory::new("shortcut");
    let executable = std::env::current_exe().expect("current test image is available");
    let link = directory.path().join("Anodrel Arguments Test.lnk");
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test")
        .expect("fixed test identity is valid");
    replace_link(
        &executable,
        executable.parent().expect("test image has a parent"),
        &arguments,
        &link,
    )
    .expect("direct Shell Link persistence succeeds");
    assert_eq!(
        super::com::read_persisted_arguments(&link).expect("link arguments are readable"),
        "--product-launch org.anodrel.shortcut-test"
    );
}
