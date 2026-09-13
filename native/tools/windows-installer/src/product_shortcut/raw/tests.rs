//! Direct Windows Shell Link checks using a temporary ordinary directory.

use super::{ProductLaunchArguments, TemporaryFile, icon, remove_regular_link, replace_link};
use crate::test_support::TestDirectory;

#[test]
fn writes_one_shell_link_in_a_regular_temporary_directory() {
    let directory = TestDirectory::new("shortcut");
    let executable = std::env::current_exe().expect("current test image is available");
    let link = directory.path().join("Anodrel Test.lnk");
    let icon = icon::replace(directory.path()).unwrap();
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test")
        .expect("fixed test identity is valid");
    replace_link(
        &executable,
        executable.parent().expect("test image has a parent"),
        &arguments,
        &icon,
        &link,
    )
    .expect("direct Shell Link persistence succeeds");
    assert!(link.is_file());
}

#[test]
fn stages_shell_links_with_a_link_extension() {
    let directory = TestDirectory::new("shortcut-stage");
    let temporary = TemporaryFile::create(directory.path(), "tmp.lnk")
        .expect("a private Shell Link stage is created");

    assert!(
        temporary.path().to_string_lossy().ends_with(".tmp.lnk"),
        "the private stage is recognizable as a Shell Link"
    );
}

#[test]
fn writes_one_shell_link_from_canonical_windows_paths() {
    let directory = TestDirectory::new("shortcut-canonical");
    let executable = std::env::current_exe().expect("current test image is available");
    let executable = std::fs::canonicalize(executable).expect("test image canonicalizes");
    let parent = executable.parent().expect("test image has a parent");
    let link = directory.path().join("Anodrel Canonical Test.lnk");
    let icon = icon::replace(directory.path()).unwrap();
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test")
        .expect("fixed test identity is valid");

    replace_link(&executable, parent, &arguments, &icon, &link)
        .expect("canonical Windows paths persist a Shell Link");
    assert!(link.is_file());
}

#[test]
fn removes_only_the_regular_shell_link_it_just_created() {
    let directory = TestDirectory::new("shortcut");
    let executable = std::env::current_exe().expect("current test image is available");
    let link = directory.path().join("Anodrel Test.lnk");
    let icon = icon::replace(directory.path()).unwrap();
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test")
        .expect("fixed test identity is valid");
    replace_link(
        &executable,
        executable.parent().expect("test image has a parent"),
        &arguments,
        &icon,
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
    let icon = icon::replace(directory.path()).unwrap();
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test")
        .expect("fixed test identity is valid");
    replace_link(
        &executable,
        executable.parent().expect("test image has a parent"),
        &arguments,
        &icon,
        &link,
    )
    .expect("direct Shell Link persistence succeeds");
    assert_eq!(
        super::com::read_persisted_arguments(&link).expect("link arguments are readable"),
        "--product-launch org.anodrel.shortcut-test"
    );
}

#[test]
fn persists_the_fixed_host_owned_icon_location() {
    let directory = TestDirectory::new("shortcut-icon-location");
    let executable = std::env::current_exe().unwrap();
    let icon = icon::replace(directory.path()).unwrap();
    let link = directory.path().join("Anodrel Icon Test.lnk");
    let arguments = ProductLaunchArguments::for_application("org.anodrel.shortcut-test").unwrap();
    replace_link(
        &executable,
        executable.parent().unwrap(),
        &arguments,
        &icon,
        &link,
    )
    .unwrap();
    let (persisted, index) = super::com::read_persisted_icon_location(&link).unwrap();
    assert_eq!(persisted, icon);
    assert_eq!(index, 0);
}
