//! UI Automation HRESULT, property, scope, and identifier constants.

pub(crate) type Hresult = i32;

pub(crate) const S_OK: Hresult = 0;
pub(crate) const E_NOINTERFACE: Hresult = -2_147_467_262;
pub(crate) const E_POINTER: Hresult = -2_147_467_261;
pub(crate) const E_FAIL: Hresult = -2_147_467_259;
pub(crate) const COINIT_MULTITHREADED: u32 = 0;
pub(crate) const CLSCTX_INPROC_SERVER: u32 = 1;
pub(crate) const VT_I4: u16 = 3;
pub(crate) const VT_BSTR: u16 = 8;
pub(crate) const VT_BOOL: u16 = 11;

pub(crate) const UIA_CONTROL_TYPE_PROPERTY_ID: i32 = 30_003;
pub(crate) const UIA_NAME_PROPERTY_ID: i32 = 30_005;
pub(crate) const UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID: i32 = 30_008;
pub(crate) const UIA_AUTOMATION_ID_PROPERTY_ID: i32 = 30_011;
pub(crate) const UIA_LIVE_REGION_CHANGED_EVENT_ID: i32 = 20_024;
pub(crate) const UIA_INVOKE_PATTERN_ID: i32 = 10_000;
pub(crate) const UIA_VALUE_PATTERN_ID: i32 = 10_002;
pub(crate) const TREE_SCOPE_ELEMENT: i32 = 1;
pub(crate) const TREE_SCOPE_SUBTREE: i32 = 7;
pub(crate) const STRUCTURE_CHANGE_CHILDREN_INVALIDATED: i32 = 2;

/// The UI Automation client coclass from `UIAutomationClient.h`.
pub(crate) const CLSID_C_UI_AUTOMATION: Guid = Guid::new(
    0xff48_dba4,
    0x60ef,
    0x4201,
    [0xaa, 0x87, 0x54, 0x10, 0x3e, 0xef, 0x59, 0x4e],
);

/// The `IUIAutomation` interface from `UIAutomationClient.h`.
pub(crate) const IID_I_UI_AUTOMATION: Guid = Guid::new(
    0x30cb_e57d,
    0xd9d0,
    0x452a,
    [0xab, 0x13, 0x7a, 0xc5, 0xac, 0x48, 0x25, 0xee],
);

/// The base COM interface every client callback must answer.
pub(crate) const IID_I_UNKNOWN: Guid = Guid::new(
    0x0000_0000,
    0x0000,
    0x0000,
    [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
);

/// The `IUIAutomationFocusChangedEventHandler` callback interface from
/// `UIAutomationClient.h`.
pub(crate) const IID_I_UI_AUTOMATION_FOCUS_CHANGED_EVENT_HANDLER: Guid = Guid::new(
    0xc270_f6b5,
    0x5c69,
    0x4290,
    [0x97, 0x45, 0x7a, 0x7f, 0x97, 0x16, 0x94, 0x68],
);

/// The `IUIAutomationEventHandler` callback interface from
/// `UIAutomationClient.h`.
pub(crate) const IID_I_UI_AUTOMATION_EVENT_HANDLER: Guid = Guid::new(
    0x146c_3c17,
    0xf12e,
    0x4e22,
    [0x8c, 0x27, 0xf8, 0x94, 0xb9, 0xb7, 0x9c, 0x69],
);

/// The `IUIAutomationStructureChangedEventHandler` callback interface from
/// `UIAutomationClient.h`.
pub(crate) const IID_I_UI_AUTOMATION_STRUCTURE_CHANGED_EVENT_HANDLER: Guid = Guid::new(
    0xe81d_1b4e,
    0x11c5,
    0x42f8,
    [0x97, 0x54, 0xe7, 0x03, 0x6c, 0x79, 0xf0, 0x54],
);

/// The client-side `IUIAutomationValuePattern` interface from
/// `UIAutomationClient.h`.
pub(crate) const IID_I_UI_AUTOMATION_VALUE_PATTERN: Guid = Guid::new(
    0xa94c_d8b1,
    0x0844,
    0x4cd6,
    [0x9d, 0x2d, 0x64, 0x05, 0x37, 0xab, 0x39, 0xe9],
);

/// The client-side `IUIAutomationInvokePattern` interface from
/// `UIAutomationClient.h`.
pub(crate) const IID_I_UI_AUTOMATION_INVOKE_PATTERN: Guid = Guid::new(
    0xfb37_7fbe,
    0x8ea6,
    0x46d5,
    [0x9c, 0x73, 0x64, 0x99, 0x64, 0x2d, 0x30, 0x59],
);

#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct Guid {
    pub(crate) data1: u32,
    pub(crate) data2: u16,
    pub(crate) data3: u16,
    pub(crate) data4: [u8; 8],
}

impl Guid {
    const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }
}
