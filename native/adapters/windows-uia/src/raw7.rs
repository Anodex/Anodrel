//! UI Automation ScrollItem-pattern bindings.
//!
//! These exact Windows SDK identifiers are isolated from the provider's COM
//! object so the host-owned route stays ordinary Rust data. See Decision 0098.

use crate::raw::Guid;

/// `UIA_ScrollItemPatternId`.
pub const UIA_SCROLL_ITEM_PATTERN_ID: i32 = 10_017;

/// `IID_IScrollItemProvider`.
pub const IID_ISCROLL_ITEM_PROVIDER: Guid = Guid {
    data1: 0x2360_C714,
    data2: 0x4BF1,
    data3: 0x4B26,
    data4: [0xBA, 0x65, 0x9B, 0x21, 0x31, 0x61, 0x27, 0xEB],
};

#[cfg(test)]
mod tests {
    use super::{IID_ISCROLL_ITEM_PROVIDER, UIA_SCROLL_ITEM_PATTERN_ID};

    #[test]
    fn scroll_item_identifiers_match_the_windows_sdk() {
        assert_eq!(UIA_SCROLL_ITEM_PATTERN_ID, 10_017);
        assert_eq!(IID_ISCROLL_ITEM_PROVIDER.data1, 0x2360_C714);
        assert_eq!(IID_ISCROLL_ITEM_PROVIDER.data2, 0x4BF1);
        assert_eq!(IID_ISCROLL_ITEM_PROVIDER.data3, 0x4B26);
        assert_eq!(
            IID_ISCROLL_ITEM_PROVIDER.data4,
            [0xBA, 0x65, 0x9B, 0x21, 0x31, 0x61, 0x27, 0xEB]
        );
    }
}
