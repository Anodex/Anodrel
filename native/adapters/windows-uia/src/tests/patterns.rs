//! UI Automation pattern callback verification.

use super::*;

#[test]
fn an_enabled_authenticated_button_exposes_and_queues_only_invoke() {
    let (provider, mailbox, revision) = invokable_child();
    // SAFETY: this test owns one live provider and writable output slots.
    unsafe {
        let (result, queried) = query_simple(provider, &IID_IINVOKE_PROVIDER);
        assert_eq!(result, S_OK);
        assert!(!queried.is_null());
        release_provider(provider);

        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let mut pattern = ptr::null_mut();
        assert_eq!(
            crate::get_pattern_provider(simple, UIA_INVOKE_PATTERN_ID, &mut pattern),
            S_OK
        );
        assert!(!pattern.is_null());

        let vtable = *pattern.cast::<*const InvokeVtbl>();
        assert_eq!(((*vtable).invoke)(pattern), S_OK);
        release_provider(provider);
        release_provider(provider);
    }

    let batch = mailbox.drain();
    assert_eq!(batch.dropped(), 0);
    let candidates = batch.into_candidates();
    assert_eq!(candidates.len(), 1);
    let SessionInteractionCandidate::Ui(candidate) =
        candidates.into_iter().next().expect("one action")
    else {
        panic!("Invoke must produce a document candidate");
    };
    let (candidate_revision, event) = candidate.into_parts();
    assert_eq!(candidate_revision, revision);
    assert_eq!(
        event,
        anodrel_ui::UiEvent::ActionInvoked(
            anodrel_ui::ElementId::new("continue").expect("fixed ID is valid")
        )
    );
}

#[test]
fn a_field_exposes_its_value_without_an_automation_write() {
    let provider = value_child();
    // SAFETY: this test owns one live provider and writable output slots.
    unsafe {
        let (result, queried) = query_simple(provider, &IID_IVALUE_PROVIDER);
        assert_eq!(result, S_OK);
        assert!(!queried.is_null());
        release_provider(provider);

        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let mut pattern = ptr::null_mut();
        assert_eq!(
            crate::get_pattern_provider(simple, UIA_VALUE_PATTERN_ID, &mut pattern),
            S_OK
        );
        assert!(!pattern.is_null());

        let vtable = *pattern.cast::<*const ValueVtbl>();
        let mut value = ptr::null_mut();
        assert_eq!(((*vtable).get_value)(pattern, &mut value), S_OK);
        assert!(!value.is_null());
        assert_eq!(crate::raw::copy_and_free_bstr(value), "Ada");

        let mut read_only = 0;
        assert_eq!(((*vtable).get_is_read_only)(pattern, &mut read_only), S_OK);
        assert_eq!(read_only, 1);
        assert_eq!(
            ((*vtable).set_value)(pattern, ptr::null()),
            UIA_E_NOTSUPPORTED
        );
        release_provider(provider);
        release_provider(provider);
    }
}

#[test]
fn only_the_selected_overflowing_group_exposes_standard_vertical_scroll() {
    let (provider, commands) = scrollable_child("viewport");
    // SAFETY: this test owns one live provider, the interface pointers it
    // queries, and all output slots passed to its COM calls.
    unsafe {
        let (result, queried) = query_simple(provider, &IID_ISCROLL_PROVIDER);
        assert_eq!(result, S_OK);
        assert!(!queried.is_null());
        release_provider(provider);

        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let mut pattern = ptr::null_mut();
        assert_eq!(
            crate::get_pattern_provider(simple, UIA_SCROLL_PATTERN_ID, &mut pattern),
            S_OK
        );
        assert!(!pattern.is_null());

        let vtable = *pattern.cast::<*const ScrollVtbl>();
        let mut horizontal = 0.0;
        let mut vertical = 0.0;
        let mut view = 0.0;
        let mut horizontal_enabled = 1;
        let mut vertical_enabled = 0;
        assert_eq!(
            ((*vtable).get_horizontal_scroll_percent)(pattern, &mut horizontal),
            S_OK
        );
        assert_eq!(
            ((*vtable).get_vertical_scroll_percent)(pattern, &mut vertical),
            S_OK
        );
        assert_eq!(((*vtable).get_vertical_view_size)(pattern, &mut view), S_OK);
        assert_eq!(
            ((*vtable).get_horizontally_scrollable)(pattern, &mut horizontal_enabled),
            S_OK
        );
        assert_eq!(
            ((*vtable).get_vertically_scrollable)(pattern, &mut vertical_enabled),
            S_OK
        );
        assert_eq!(horizontal, -1.0);
        assert_eq!(vertical, 0.0);
        assert_eq!(view, 50.0);
        assert_eq!(horizontal_enabled, 0);
        assert_eq!(vertical_enabled, 1);

        let mut property = crate::Variant::empty();
        assert_eq!(
            crate::get_property_value(simple, 30_055, &mut property),
            S_OK
        );
        assert_eq!(property.double_value(), Some(0.0));

        assert_eq!(((*vtable).scroll)(pattern, 2, 4), S_OK);
        assert_eq!(((*vtable).set_scroll_percent)(pattern, -1.0, 37.5), S_OK);
        assert_eq!(((*vtable).scroll)(pattern, 4, 4), UIA_E_NOTSUPPORTED);
        assert_eq!(
            ((*vtable).set_scroll_percent)(pattern, -1.0, f64::NAN),
            UIA_E_NOTSUPPORTED
        );
        release_provider(provider);
        release_provider(provider);
    }
    assert_eq!(
        *commands.lock().expect("test recording lock is available"),
        vec![
            UiAutomationScrollCommand::Line { forward: true },
            UiAutomationScrollCommand::Percent { percent: 37.5 },
        ]
    );
}

#[test]
fn an_offscreen_scroll_descendant_exposes_only_scroll_item() {
    let (provider, commands) = scrollable_child("three");
    // SAFETY: this test owns one live provider, the interface pointers it
    // queries, and all output slots passed to its COM calls.
    unsafe {
        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let mut offscreen = crate::Variant::empty();
        assert_eq!(
            crate::get_property_value(simple, property::IS_OFFSCREEN, &mut offscreen),
            S_OK
        );
        assert_eq!(offscreen.boolean_value(), Some(true));

        let (result, rejected) = query_simple(provider, &IID_IINVOKE_PROVIDER);
        assert_eq!(result, E_NOINTERFACE);
        assert!(rejected.is_null());

        let (result, queried) = query_simple(provider, &IID_ISCROLL_ITEM_PROVIDER);
        assert_eq!(result, S_OK);
        assert!(!queried.is_null());
        release_provider(provider);

        let mut pattern = ptr::null_mut();
        assert_eq!(
            crate::get_pattern_provider(simple, UIA_SCROLL_ITEM_PATTERN_ID, &mut pattern),
            S_OK
        );
        assert!(!pattern.is_null());

        let vtable = *pattern.cast::<*const ScrollItemVtbl>();
        assert_eq!(((*vtable).scroll_into_view)(pattern), S_OK);
        release_provider(provider);
        release_provider(provider);
    }
    assert_eq!(
        *commands.lock().expect("test recording lock is available"),
        vec![UiAutomationScrollCommand::ScrollIntoView {
            item: anodrel_ui::ElementId::new("three").expect("fixed ID is valid"),
        }]
    );
}
