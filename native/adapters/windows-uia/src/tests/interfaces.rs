//! COM interface and fragment-navigation verification.

use super::*;

#[test]
fn the_window_answers_all_three_interfaces() {
    let provider = root();
    for iid in [
        IID_IUNKNOWN,
        IID_IRAW_ELEMENT_PROVIDER_SIMPLE,
        IID_IRAW_ELEMENT_PROVIDER_FRAGMENT,
        IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT,
    ] {
        // SAFETY: a live provider.
        let (result, out) = unsafe { query_simple(provider, &iid) };
        assert_eq!(result, S_OK, "{iid:?}");
        assert!(!out.is_null());
        // SAFETY: releasing the reference the query added.
        unsafe { release_provider(provider) };
    }
    // SAFETY: releasing the creation reference.
    unsafe { release_provider(provider) };
}

#[test]
fn an_element_is_not_a_fragment_root() {
    // Only the window roots the tree. An element claiming otherwise would
    // make navigation ambiguous.
    let provider = child();
    // SAFETY: a live provider.
    let (result, out) = unsafe { query_simple(provider, &IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT) };
    assert_eq!(result, E_NOINTERFACE);
    assert!(out.is_null());

    // SAFETY: a live provider.
    let (result, _) = unsafe { query_simple(provider, &IID_IRAW_ELEMENT_PROVIDER_FRAGMENT) };
    assert_eq!(result, S_OK);
    // SAFETY: releasing the query's reference, then the creation one.
    unsafe {
        release_provider(provider);
        release_provider(provider);
    }
}

#[test]
fn fragment_navigation_follows_the_published_hierarchy() {
    let root = hierarchy_root();
    let mut group = ptr::null_mut();
    let mut detail = ptr::null_mut();
    // SAFETY: each interface belongs to a live provider owned by this test,
    // and every output slot is writable. Each successful navigation adds a
    // reference that the matching release below returns.
    unsafe {
        let root_fragment = (&raw mut (*root).fragment).cast::<c_void>();
        assert_eq!(
            crate::navigate(
                root_fragment,
                crate::raw2::direction::FIRST_CHILD,
                &mut group
            ),
            S_OK
        );
        assert_eq!((*crate::fragment_of(group)).element, Some(0));

        assert_eq!(
            crate::navigate(group, crate::raw2::direction::FIRST_CHILD, &mut detail),
            S_OK
        );
        assert_eq!((*crate::fragment_of(detail)).element, Some(1));

        let mut nested_group = ptr::null_mut();
        assert_eq!(
            crate::navigate(
                detail,
                crate::raw2::direction::NEXT_SIBLING,
                &mut nested_group
            ),
            S_OK
        );
        assert_eq!((*crate::fragment_of(nested_group)).element, Some(2));

        let mut nested_child = ptr::null_mut();
        assert_eq!(
            crate::navigate(
                nested_group,
                crate::raw2::direction::FIRST_CHILD,
                &mut nested_child,
            ),
            S_OK
        );
        assert_eq!((*crate::fragment_of(nested_child)).element, Some(3));

        release_provider(crate::fragment_of(nested_child));
        release_provider(crate::fragment_of(nested_group));
        release_provider(crate::fragment_of(detail));
        release_provider(crate::fragment_of(group));
        release_provider(root);
    }
}

#[test]
fn a_refused_interface_clears_its_output() {
    let provider = root();
    let unsupported = Guid {
        data1: 1,
        data2: 2,
        data3: 3,
        data4: [4; 8],
    };
    // SAFETY: a live provider.
    let (result, out) = unsafe { query_simple(provider, &unsupported) };
    assert_eq!(result, E_NOINTERFACE);
    assert!(out.is_null());
    // SAFETY: releasing the creation reference.
    unsafe { release_provider(provider) };
}
