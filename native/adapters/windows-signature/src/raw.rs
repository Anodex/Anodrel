//! Raw Windows trust and certificate bindings.
//!
//! All unsafe work stays in this module. The public parent module exposes only
//! a fixed fingerprint and a safe failure catalogue.

use std::{ffi::c_void, mem, os::windows::ffi::OsStrExt, path::Path, ptr};

use crate::SignatureError;

type Bool = i32;
type Dword = u32;
type Handle = isize;
type Hmodule = isize;

const WTD_UI_NONE: Dword = 2;
const WTD_REVOKE_NONE: Dword = 0;
const WTD_CHOICE_FILE: Dword = 1;
const WTD_STATEACTION_VERIFY: Dword = 1;
const WTD_STATEACTION_CLOSE: Dword = 2;
const CERT_SHA256_HASH_PROP_ID: Dword = 107;

/// Authenticode policy identifier published by the Windows SDK.
const GENERIC_VERIFY_V2: Guid = Guid {
    data1: 0x00AA_C56B,
    data2: 0xCD44,
    data3: 0x11D0,
    data4: [0x8C, 0xC2, 0x00, 0xC0, 0x4F, 0xC2, 0x95, 0xEE],
};

#[repr(C)]
struct Guid {
    data1: Dword,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct WinTrustFileInfo {
    cb_struct: Dword,
    file_path: *const u16,
    file: Handle,
    known_subject: *const Guid,
}

#[repr(C)]
struct WinTrustData {
    cb_struct: Dword,
    policy_callback_data: *mut c_void,
    sip_client_data: *mut c_void,
    ui_choice: Dword,
    revocation_checks: Dword,
    union_choice: Dword,
    file: *mut WinTrustFileInfo,
    state_action: Dword,
    state_data: Handle,
    url_reference: *const u16,
    provider_flags: Dword,
    ui_context: Dword,
    signature_settings: *mut c_void,
}

/// The provider certificate layout starts with its size and certificate context.
#[repr(C)]
struct CryptProviderCert {
    _cb_struct: Dword,
    certificate: *const CertContext,
}

/// Certificate contexts remain owned by the Windows trust provider.
#[repr(C)]
struct CertContext {
    _reserved: [u8; 0],
}

type ProviderFromState = unsafe extern "system" fn(Handle) -> *mut c_void;
type SignerFromChain = unsafe extern "system" fn(*mut c_void, Dword, Bool, Dword) -> *mut c_void;
type CertificateFromChain = unsafe extern "system" fn(*mut c_void, Dword) -> *mut CryptProviderCert;

#[link(name = "wintrust")]
unsafe extern "system" {
    fn WinVerifyTrust(window: Handle, action: *const Guid, trust_data: *mut WinTrustData) -> i32;
}

#[link(name = "crypt32")]
unsafe extern "system" {
    fn CertGetCertificateContextProperty(
        certificate: *const CertContext,
        property: Dword,
        data: *mut c_void,
        data_size: *mut Dword,
    ) -> Bool;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> Hmodule;
    fn GetProcAddress(module: Hmodule, name: *const u8) -> *const c_void;
    fn FreeLibrary(module: Hmodule) -> Bool;
}

/// Verifies a file and extracts its leaf certificate fingerprint while the
/// WinTrust state remains open.
pub(super) fn verify_embedded_signature(path: &Path) -> Result<[u8; 32], SignatureError> {
    let path = wide_null(path).ok_or(SignatureError::InvalidPath)?;
    let mut file = WinTrustFileInfo {
        cb_struct: size_of::<WinTrustFileInfo>(),
        file_path: path.as_ptr(),
        file: 0,
        known_subject: ptr::null(),
    };
    let mut data = WinTrustData {
        cb_struct: size_of::<WinTrustData>(),
        policy_callback_data: ptr::null_mut(),
        sip_client_data: ptr::null_mut(),
        ui_choice: WTD_UI_NONE,
        revocation_checks: WTD_REVOKE_NONE,
        union_choice: WTD_CHOICE_FILE,
        file: &mut file,
        state_action: WTD_STATEACTION_VERIFY,
        state_data: 0,
        url_reference: ptr::null(),
        provider_flags: 0,
        ui_context: 0,
        signature_settings: ptr::null_mut(),
    };

    let helpers = TrustHelpers::load()?;
    // SAFETY: file and data have the documented WinTrust layouts, point to
    // valid memory for the call, and remain live until the paired close call.
    let status = unsafe { WinVerifyTrust(0, &GENERIC_VERIFY_V2, &mut data) };
    if status != 0 {
        close_state(&mut data);
        return Err(SignatureError::TrustRejected);
    }

    let result = leaf_fingerprint(&helpers, data.state_data);
    close_state(&mut data);
    result
}

fn leaf_fingerprint(helpers: &TrustHelpers, state: Handle) -> Result<[u8; 32], SignatureError> {
    // SAFETY: state was returned by a successful WinVerifyTrust call and stays
    // open until the caller closes its state after this function returns.
    let provider = unsafe { (helpers.provider_from_state)(state) };
    if provider.is_null() {
        return Err(SignatureError::ProviderStateUnavailable);
    }
    // SAFETY: provider is owned by the active trust state; zero selects the
    // primary signer rather than any countersigner.
    let signer = unsafe { (helpers.signer_from_chain)(provider, 0, 0, 0) };
    if signer.is_null() {
        return Err(SignatureError::SignerUnavailable);
    }
    // SAFETY: signer belongs to the active provider state; zero selects its
    // leaf certificate.
    let certificate = unsafe { (helpers.certificate_from_chain)(signer, 0) };
    if certificate.is_null() || unsafe { (*certificate).certificate.is_null() } {
        return Err(SignatureError::CertificateUnavailable);
    }
    // SAFETY: certificate belongs to the active provider state and remains
    // valid until that state is closed after this function returns.
    unsafe { certificate_fingerprint((*certificate).certificate) }
}

unsafe fn certificate_fingerprint(
    certificate: *const CertContext,
) -> Result<[u8; 32], SignatureError> {
    let mut size = 0;
    // SAFETY: certificate came from the active WinTrust state; the null buffer
    // asks CryptoAPI for the exact property size.
    if unsafe {
        CertGetCertificateContextProperty(
            certificate,
            CERT_SHA256_HASH_PROP_ID,
            ptr::null_mut(),
            &mut size,
        )
    } == 0
        || size != 32
    {
        return Err(SignatureError::FingerprintUnavailable);
    }

    let mut fingerprint = [0_u8; 32];
    // SAFETY: fingerprint has exactly the size CryptoAPI reported, and the
    // certificate remains valid under the open trust state.
    if unsafe {
        CertGetCertificateContextProperty(
            certificate,
            CERT_SHA256_HASH_PROP_ID,
            fingerprint.as_mut_ptr().cast(),
            &mut size,
        )
    } == 0
    {
        return Err(SignatureError::FingerprintUnavailable);
    }
    Ok(fingerprint)
}

fn close_state(data: &mut WinTrustData) {
    if data.state_data == 0 {
        return;
    }
    data.state_action = WTD_STATEACTION_CLOSE;
    // SAFETY: data is the same initialized WinTrust structure used for the
    // successful verification and its state is being released exactly once.
    let _ = unsafe { WinVerifyTrust(0, &GENERIC_VERIFY_V2, data) };
    data.state_data = 0;
}

struct TrustHelpers {
    _library: DynamicLibrary,
    provider_from_state: ProviderFromState,
    signer_from_chain: SignerFromChain,
    certificate_from_chain: CertificateFromChain,
}

impl TrustHelpers {
    fn load() -> Result<Self, SignatureError> {
        let library = DynamicLibrary::load("wintrust.dll")?;
        Ok(Self {
            provider_from_state: library.symbol(b"WTHelperProvDataFromStateData\0")?,
            signer_from_chain: library.symbol(b"WTHelperGetProvSignerFromChain\0")?,
            certificate_from_chain: library.symbol(b"WTHelperGetProvCertFromChain\0")?,
            _library: library,
        })
    }
}

struct DynamicLibrary(Hmodule);

impl DynamicLibrary {
    fn load(name: &str) -> Result<Self, SignatureError> {
        let name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: name is a null-terminated UTF-16 module name that stays live
        // through the call. The returned handle is released by Drop.
        let module = unsafe { LoadLibraryW(name.as_ptr()) };
        if module == 0 {
            Err(SignatureError::ProviderStateUnavailable)
        } else {
            Ok(Self(module))
        }
    }

    fn symbol<T>(&self, name: &[u8]) -> Result<T, SignatureError>
    where
        T: Copy,
    {
        // SAFETY: name is an ASCII symbol name with its terminating null byte;
        // self owns a loaded module for the duration of this lookup.
        let address = unsafe { GetProcAddress(self.0, name.as_ptr()) };
        if address.is_null() {
            return Err(SignatureError::ProviderStateUnavailable);
        }
        // SAFETY: each caller requests the documented function-pointer
        // signature for its named WinTrust export.
        Ok(unsafe { mem::transmute_copy(&address) })
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        // SAFETY: this module handle was returned by LoadLibraryW and is owned
        // by this value. It is released only after all helper calls finish.
        let _ = unsafe { FreeLibrary(self.0) };
    }
}

fn wide_null(path: &Path) -> Option<Vec<u16>> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() || wide.contains(&0) {
        return None;
    }
    wide.push(0);
    Some(wide)
}

const fn size_of<T>() -> Dword {
    mem::size_of::<T>() as Dword
}
