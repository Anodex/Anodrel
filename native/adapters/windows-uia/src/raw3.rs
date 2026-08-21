//! UI Automation Invoke-pattern bindings.
//!
//! This stays separate from the provider's fragment bindings so the one action
//! surface is explicit: one pattern ID, one interface identifier, and no
//! client-side UI Automation API.

use crate::raw::Guid;

/// `UIA_InvokePatternId`.
pub const UIA_INVOKE_PATTERN_ID: i32 = 10_000;

/// `IID_IInvokeProvider`.
pub const IID_IINVOKE_PROVIDER: Guid = Guid {
    data1: 0x54FC_B24B,
    data2: 0xE18E,
    data3: 0x47A2,
    data4: [0xB4, 0xD3, 0xEC, 0xCB, 0xE7, 0x75, 0x99, 0xA2],
};

#[cfg(test)]
mod tests {
    use super::{IID_IINVOKE_PROVIDER, UIA_INVOKE_PATTERN_ID};

    #[test]
    fn the_invoke_identifiers_match_ui_automation() {
        assert_eq!(UIA_INVOKE_PATTERN_ID, 10_000);
        assert_eq!(IID_IINVOKE_PROVIDER.data1, 0x54FC_B24B);
        assert_eq!(
            IID_IINVOKE_PROVIDER.data4,
            [0xB4, 0xD3, 0xEC, 0xCB, 0xE7, 0x75, 0x99, 0xA2]
        );
    }
}
