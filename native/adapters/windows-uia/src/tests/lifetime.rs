//! Provider lifetime, focus, and failure-boundary verification.

use super::*;

#[test]
fn reference_counting_frees_the_object_exactly_once() {
    let provider = root();
    // SAFETY: a live provider held by this test.
    unsafe {
        assert_eq!(increment(provider), 2);
        assert_eq!(release_provider(provider), 1);
        assert_eq!(release_provider(provider), 0);
    }
}

#[test]
fn focus_requires_a_live_visible_provider_and_updates_its_snapshot() {
    // SAFETY: a null interface pointer has no live provider to recover.
    let result = unsafe { set_focus(ptr::null_mut()) };
    assert_eq!(result, E_POINTER);

    let root = root();
    // SAFETY: this test owns a live root provider, which is not a child
    // focus target.
    let result = unsafe { set_focus((&raw mut (*root).fragment).cast::<c_void>()) };
    assert_eq!(result, UIA_E_NOTSUPPORTED);
    assert!(result < 0);
    // SAFETY: releasing this test's creation reference.
    unsafe { release_provider(root) };

    let provider = focusable_child();
    // SAFETY: the provider is live and the route completes synchronously
    // in this test without creating a native input path.
    unsafe {
        let fragment = (&raw mut (*provider).fragment).cast::<c_void>();
        assert_eq!(set_focus(fragment), S_OK);
        assert_eq!((*provider).tree.focused(), Some(0));
        let vtable = *fragment.cast::<*const FragmentVtbl>();
        assert_eq!(((*vtable).set_focus)(fragment), S_OK);
        release_provider(provider);
    }
}

#[test]
fn the_root_returns_only_its_published_focus_snapshot() {
    let provider = focused_root();
    let mut focused = ptr::null_mut();
    // SAFETY: this test owns a live root provider and a writable output
    // slot. The returned fragment owns a separate reference.
    unsafe {
        let root = (&raw mut (*provider).fragment_root).cast::<c_void>();
        assert_eq!(crate::get_focus(root, &mut focused), S_OK);
        assert!(!focused.is_null());
        let focused_provider = crate::fragment_of(focused);
        assert_eq!((*focused_provider).element, Some(0));
        release_provider(focused_provider);
        release_provider(provider);
    }

    let provider = root();
    let mut focused = ptr::dangling_mut::<c_void>();
    // SAFETY: the empty tree has no focused child and the output is
    // writable, so the method must clear it rather than preserve a value.
    unsafe {
        let root = (&raw mut (*provider).fragment_root).cast::<c_void>();
        assert_eq!(crate::get_focus(root, &mut focused), S_OK);
        assert!(focused.is_null());
        release_provider(provider);
    }
}

#[test]
fn a_panicking_method_body_fails_instead_of_aborting() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = contain(|| panic!("a provider defect must not abort the host"));
    std::panic::set_hook(previous);
    assert_eq!(result, crate::E_FAIL);
    assert_eq!(contain(|| S_OK), S_OK);
}

#[test]
fn every_navigation_method_rejects_a_null_output() {
    let provider = root();
    let fragment = (&raw mut unsafe { &mut *provider }.fragment).cast::<c_void>();
    // SAFETY: a live provider with deliberately null output slots.
    unsafe {
        assert_eq!(crate::navigate(fragment, 3, ptr::null_mut()), E_POINTER);
        assert_eq!(
            crate::get_pattern_provider(
                (&raw mut (*provider).simple).cast::<c_void>(),
                UIA_INVOKE_PATTERN_ID,
                ptr::null_mut(),
            ),
            E_POINTER
        );
        assert_eq!(crate::get_runtime_id(fragment, ptr::null_mut()), E_POINTER);
        assert_eq!(
            crate::get_bounding_rectangle(fragment, ptr::null_mut()),
            E_POINTER
        );
        assert_eq!(
            crate::get_focus(
                (&raw mut (*provider).fragment_root).cast::<c_void>(),
                ptr::null_mut(),
            ),
            E_POINTER
        );
        assert_eq!(
            crate::get_value(
                (&raw mut (*provider).value).cast::<c_void>(),
                ptr::null_mut(),
            ),
            E_POINTER
        );
        assert_eq!(
            crate::get_is_read_only(
                (&raw mut (*provider).value).cast::<c_void>(),
                ptr::null_mut(),
            ),
            E_POINTER
        );
        release_provider(provider);
    }
}
