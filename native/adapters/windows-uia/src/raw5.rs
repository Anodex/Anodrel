//! UI Automation Value-pattern bindings.
//!
//! A Value provider is intentionally distinct from the semantic tree and from
//! the Invoke binding: it reads one host-owned field snapshot and offers no
//! automation write. See Decision 0071.

use crate::raw::Guid;

/// `UIA_ValuePatternId`.
pub const UIA_VALUE_PATTERN_ID: i32 = 10_002;

/// `UIA_ValueValuePropertyId`.
pub const UIA_VALUE_VALUE_PROPERTY_ID: i32 = 30_045;

/// `UIA_ValueIsReadOnlyPropertyId`.
pub const UIA_VALUE_IS_READ_ONLY_PROPERTY_ID: i32 = 30_046;

/// `IID_IValueProvider`.
pub const IID_IVALUE_PROVIDER: Guid = Guid {
    data1: 0xC793_5180,
    data2: 0x6FB3,
    data3: 0x4201,
    data4: [0xB1, 0x74, 0x7D, 0xF7, 0x3A, 0xDB, 0xF6, 0x4A],
};

#[cfg(test)]
mod tests {
    use super::{
        IID_IVALUE_PROVIDER, UIA_VALUE_IS_READ_ONLY_PROPERTY_ID, UIA_VALUE_PATTERN_ID,
        UIA_VALUE_VALUE_PROPERTY_ID,
    };

    #[test]
    fn the_value_identifiers_match_ui_automation() {
        assert_eq!(UIA_VALUE_PATTERN_ID, 10_002);
        assert_eq!(UIA_VALUE_VALUE_PROPERTY_ID, 30_045);
        assert_eq!(UIA_VALUE_IS_READ_ONLY_PROPERTY_ID, 30_046);
        assert_eq!(IID_IVALUE_PROVIDER.data1, 0xC793_5180);
        assert_eq!(
            IID_IVALUE_PROVIDER.data4,
            [0xB1, 0x74, 0x7D, 0xF7, 0x3A, 0xDB, 0xF6, 0x4A]
        );
    }
}
