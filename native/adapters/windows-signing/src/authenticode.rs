//! Direct `SignerSignEx` composition for one caller-validated file.

use std::{
    ffi::{c_char, c_void},
    mem,
    os::windows::ffi::OsStrExt,
    path::Path,
    ptr,
};

use crate::{
    WindowsSigningError,
    certificate::{CertificateContext, CertificateStore, SelectedCertificate},
};

type Handle = *mut c_void;
type ModuleHandle = *mut c_void;

const SIGNER_SUBJECT_FILE: u32 = 1;
const SIGNER_CERT_STORE: u32 = 2;
const SIGNER_CERT_POLICY_CHAIN_NO_ROOT: u32 = 8;
const CALG_SHA_256: u32 = 0x0000_800C;
const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;

#[repr(C)]
struct SignerFileInfo {
    size: u32,
    file_name: *const u16,
    file: Handle,
}

#[repr(C)]
struct SignerSubjectInfo {
    size: u32,
    index: *mut u32,
    choice: u32,
    file_info: *mut SignerFileInfo,
}

#[repr(C)]
struct SignerCertificateStoreInfo {
    size: u32,
    certificate: *const CertificateContext,
    policy: u32,
    store: CertificateStore,
}

#[repr(C)]
struct SignerCertificate {
    size: u32,
    choice: u32,
    store_info: *mut SignerCertificateStoreInfo,
    window: Handle,
}

#[repr(C)]
struct SignerSignatureInfo {
    size: u32,
    hash_algorithm: u32,
    attribute_choice: u32,
    authcode_attributes: *mut c_void,
    authenticated_attributes: *mut c_void,
    unauthenticated_attributes: *mut c_void,
}

#[repr(C)]
struct SignerContext {
    _private: [u8; 0],
}

type SignerSignEx = unsafe extern "system" fn(
    u32,
    *mut SignerSubjectInfo,
    *mut SignerCertificate,
    *mut SignerSignatureInfo,
    *mut c_void,
    *const u16,
    *mut c_void,
    *mut c_void,
    *mut *mut SignerContext,
) -> i32;
type SignerFreeContext = unsafe extern "system" fn(*mut SignerContext) -> i32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(name: *const u16, file: Handle, flags: u32) -> ModuleHandle;
    fn GetProcAddress(module: ModuleHandle, name: *const c_char) -> *const c_void;
    fn FreeLibrary(module: ModuleHandle) -> i32;
}

/// Signs one caller-validated absolute file with one exact certificate.
pub(super) fn sign_file(path: &Path, fingerprint: [u8; 32]) -> Result<(), WindowsSigningError> {
    if !path.is_absolute() {
        return Err(WindowsSigningError::AuthenticodePathInvalid);
    }
    let certificate = SelectedCertificate::find(fingerprint)?;
    let signer = SigningLibrary::load()?;
    let image_name = wide_null(path).ok_or(WindowsSigningError::AuthenticodePathInvalid)?;
    let mut file = SignerFileInfo {
        size: size_of::<SignerFileInfo>(),
        file_name: image_name.as_ptr(),
        file: ptr::null_mut(),
    };
    let mut subject = SignerSubjectInfo {
        size: size_of::<SignerSubjectInfo>(),
        index: ptr::null_mut(),
        choice: SIGNER_SUBJECT_FILE,
        file_info: &mut file,
    };
    let mut store_info = SignerCertificateStoreInfo {
        size: size_of::<SignerCertificateStoreInfo>(),
        certificate: certificate.context(),
        policy: SIGNER_CERT_POLICY_CHAIN_NO_ROOT,
        store: certificate.store(),
    };
    let mut signing_certificate = SignerCertificate {
        size: size_of::<SignerCertificate>(),
        choice: SIGNER_CERT_STORE,
        store_info: &mut store_info,
        window: ptr::null_mut(),
    };
    let mut signature = SignerSignatureInfo {
        size: size_of::<SignerSignatureInfo>(),
        hash_algorithm: CALG_SHA_256,
        attribute_choice: 0,
        authcode_attributes: ptr::null_mut(),
        authenticated_attributes: ptr::null_mut(),
        unauthenticated_attributes: ptr::null_mut(),
    };
    let mut raw_context = ptr::null_mut();
    // SAFETY: Every structure has the documented C layout and references data
    // that remains alive throughout this synchronous call. The certificate is
    // selected from the live current-user store, and all optional pointer
    // parameters are null by contract.
    let status = unsafe {
        (signer.sign)(
            0,
            &mut subject,
            &mut signing_certificate,
            &mut signature,
            ptr::null_mut(),
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut raw_context,
        )
    };
    let context = SigningContext::new(raw_context, signer.free);
    if status != 0 || context.is_empty() {
        return Err(WindowsSigningError::AuthenticodeFailed);
    }
    Ok(())
}

struct SigningLibrary {
    module: ModuleHandle,
    sign: SignerSignEx,
    free: SignerFreeContext,
}

impl SigningLibrary {
    fn load() -> Result<Self, WindowsSigningError> {
        let module = SigningModule::load()?;
        let sign = symbol(module.0, b"SignerSignEx\0")?;
        let free = symbol(module.0, b"SignerFreeSignerContext\0")?;
        Ok(Self {
            module: module.release(),
            sign,
            free,
        })
    }
}

/// Owns a system DLL until all required exports have been resolved.
struct SigningModule(ModuleHandle);

impl SigningModule {
    fn load() -> Result<Self, WindowsSigningError> {
        let library =
            wide_null(Path::new("Mssign32.dll")).expect("the fixed signing library name is valid");
        // SAFETY: This requests only the fixed Windows system DLL and does not
        // consult the application directory or current working directory.
        let module = unsafe {
            LoadLibraryExW(
                library.as_ptr(),
                ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if module.is_null() {
            Err(WindowsSigningError::AuthenticodeUnavailable)
        } else {
            Ok(Self(module))
        }
    }

    fn release(mut self) -> ModuleHandle {
        let module = self.0;
        self.0 = ptr::null_mut();
        module
    }
}

impl Drop for SigningModule {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: an unresolved-export failure still owns the system DLL
            // module and must release it before propagating that failure.
            let _ = unsafe { FreeLibrary(self.0) };
        }
    }
}

impl Drop for SigningLibrary {
    fn drop(&mut self) {
        // SAFETY: this value owns the fixed system DLL module handle and drops
        // only after any returned signing context was freed.
        let _ = unsafe { FreeLibrary(self.module) };
    }
}

struct SigningContext {
    context: *mut SignerContext,
    free: SignerFreeContext,
}

impl SigningContext {
    fn new(context: *mut SignerContext, free: SignerFreeContext) -> Self {
        Self { context, free }
    }

    fn is_empty(&self) -> bool {
        self.context.is_null()
    }
}

impl Drop for SigningContext {
    fn drop(&mut self) {
        if !self.context.is_null() {
            // SAFETY: context was returned by SignerSignEx and this guard calls
            // its documented paired freeing function exactly once.
            let _ = unsafe { (self.free)(self.context) };
        }
    }
}

fn symbol<T>(module: ModuleHandle, name: &'static [u8]) -> Result<T, WindowsSigningError>
where
    T: Copy,
{
    // SAFETY: module is an owned loaded system DLL and name is a fixed
    // NUL-terminated ASCII export name.
    let address = unsafe { GetProcAddress(module, name.as_ptr().cast()) };
    if address.is_null() {
        return Err(WindowsSigningError::AuthenticodeUnavailable);
    }
    // SAFETY: callers supply exactly the documented ABI signature for each
    // fixed Mssign32 export requested above.
    Ok(unsafe { mem::transmute_copy(&address) })
}

fn wide_null(path: &Path) -> Option<Vec<u16>> {
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return None;
    }
    wide.push(0);
    Some(wide)
}

const fn size_of<T>() -> u32 {
    mem::size_of::<T>() as u32
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::{
        SignerCertificate, SignerCertificateStoreInfo, SignerFileInfo, SignerSignatureInfo,
        SignerSubjectInfo,
    };

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn signer_structures_match_the_documented_64_bit_windows_layouts() {
        assert_eq!(mem::size_of::<SignerFileInfo>(), 24);
        assert_eq!(mem::size_of::<SignerSubjectInfo>(), 32);
        assert_eq!(mem::size_of::<SignerCertificateStoreInfo>(), 32);
        assert_eq!(mem::size_of::<SignerCertificate>(), 24);
        assert_eq!(mem::size_of::<SignerSignatureInfo>(), 40);
    }
}
