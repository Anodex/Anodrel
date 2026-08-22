//! UI Automation Scroll-pattern bindings.
//!
//! The raw identifiers and closed movement values are isolated from the COM
//! provider so Decision 0097 can be tested against the Windows SDK without
//! creating a client-side automation dependency.

use crate::raw::Guid;

/// `UIA_ScrollPatternId`.
pub const UIA_SCROLL_PATTERN_ID: i32 = 10_004;

/// `UIA_ScrollHorizontalScrollPercentPropertyId`.
pub const UIA_SCROLL_HORIZONTAL_SCROLL_PERCENT_PROPERTY_ID: i32 = 30_053;

/// `UIA_ScrollHorizontalViewSizePropertyId`.
pub const UIA_SCROLL_HORIZONTAL_VIEW_SIZE_PROPERTY_ID: i32 = 30_054;

/// `UIA_ScrollVerticalScrollPercentPropertyId`.
pub const UIA_SCROLL_VERTICAL_SCROLL_PERCENT_PROPERTY_ID: i32 = 30_055;

/// `UIA_ScrollVerticalViewSizePropertyId`.
pub const UIA_SCROLL_VERTICAL_VIEW_SIZE_PROPERTY_ID: i32 = 30_056;

/// `UIA_ScrollHorizontallyScrollablePropertyId`.
pub const UIA_SCROLL_HORIZONTALLY_SCROLLABLE_PROPERTY_ID: i32 = 30_057;

/// `UIA_ScrollVerticallyScrollablePropertyId`.
pub const UIA_SCROLL_VERTICALLY_SCROLLABLE_PROPERTY_ID: i32 = 30_058;

/// `UIA_ScrollPatternNoScroll`.
pub const UIA_SCROLL_PATTERN_NO_SCROLL: f64 = -1.0;

/// `ScrollAmount::LargeDecrement`.
pub const SCROLL_AMOUNT_LARGE_DECREMENT: i32 = 0;

/// `ScrollAmount::SmallDecrement`.
pub const SCROLL_AMOUNT_SMALL_DECREMENT: i32 = 1;

/// `ScrollAmount::NoAmount`.
pub const SCROLL_AMOUNT_NO_AMOUNT: i32 = 2;

/// `ScrollAmount::LargeIncrement`.
pub const SCROLL_AMOUNT_LARGE_INCREMENT: i32 = 3;

/// `ScrollAmount::SmallIncrement`.
pub const SCROLL_AMOUNT_SMALL_INCREMENT: i32 = 4;

/// `IID_IScrollProvider`.
pub const IID_ISCROLL_PROVIDER: Guid = Guid {
    data1: 0xB38B_8077,
    data2: 0x1FC3,
    data3: 0x42A5,
    data4: [0x8C, 0xAE, 0xD4, 0x0C, 0x22, 0x15, 0x05, 0x5A],
};

#[cfg(test)]
mod tests {
    use super::{
        IID_ISCROLL_PROVIDER, SCROLL_AMOUNT_LARGE_DECREMENT, SCROLL_AMOUNT_LARGE_INCREMENT,
        SCROLL_AMOUNT_NO_AMOUNT, SCROLL_AMOUNT_SMALL_DECREMENT, SCROLL_AMOUNT_SMALL_INCREMENT,
        UIA_SCROLL_HORIZONTAL_SCROLL_PERCENT_PROPERTY_ID,
        UIA_SCROLL_HORIZONTAL_VIEW_SIZE_PROPERTY_ID,
        UIA_SCROLL_HORIZONTALLY_SCROLLABLE_PROPERTY_ID, UIA_SCROLL_PATTERN_ID,
        UIA_SCROLL_PATTERN_NO_SCROLL, UIA_SCROLL_VERTICAL_SCROLL_PERCENT_PROPERTY_ID,
        UIA_SCROLL_VERTICAL_VIEW_SIZE_PROPERTY_ID, UIA_SCROLL_VERTICALLY_SCROLLABLE_PROPERTY_ID,
    };

    #[test]
    fn the_scroll_identifiers_match_the_windows_sdk() {
        assert_eq!(UIA_SCROLL_PATTERN_ID, 10_004);
        assert_eq!(UIA_SCROLL_HORIZONTAL_SCROLL_PERCENT_PROPERTY_ID, 30_053);
        assert_eq!(UIA_SCROLL_HORIZONTAL_VIEW_SIZE_PROPERTY_ID, 30_054);
        assert_eq!(UIA_SCROLL_VERTICAL_SCROLL_PERCENT_PROPERTY_ID, 30_055);
        assert_eq!(UIA_SCROLL_VERTICAL_VIEW_SIZE_PROPERTY_ID, 30_056);
        assert_eq!(UIA_SCROLL_HORIZONTALLY_SCROLLABLE_PROPERTY_ID, 30_057);
        assert_eq!(UIA_SCROLL_VERTICALLY_SCROLLABLE_PROPERTY_ID, 30_058);
        assert_eq!(UIA_SCROLL_PATTERN_NO_SCROLL, -1.0);
        assert_eq!(SCROLL_AMOUNT_LARGE_DECREMENT, 0);
        assert_eq!(SCROLL_AMOUNT_SMALL_DECREMENT, 1);
        assert_eq!(SCROLL_AMOUNT_NO_AMOUNT, 2);
        assert_eq!(SCROLL_AMOUNT_LARGE_INCREMENT, 3);
        assert_eq!(SCROLL_AMOUNT_SMALL_INCREMENT, 4);
        assert_eq!(IID_ISCROLL_PROVIDER.data1, 0xB38B_8077);
        assert_eq!(
            IID_ISCROLL_PROVIDER.data4,
            [0x8C, 0xAE, 0xD4, 0x0C, 0x22, 0x15, 0x05, 0x5A]
        );
    }
}
