//! Exact current-user certificate-store selection and lifetime pairing.

use std::{ffi::c_void, ptr};

use crate::WindowsSigningError;

pub(super) type CertificateStore = *mut c_void;

const X509_ASN_ENCODING: u32 = 0x0000_0001;
const PKCS_7_ASN_ENCODING: u32 = 0x0001_0000;
const CERT_FIND_SHA256_HASH: u32 = 22 << 16;
const CERT_SHA256_HASH_PROP_ID: u32 = 107;

#[repr(C)]
pub(super) struct CertificateContext {
    _private: [u8; 0],
}

#[repr(C)]
struct CryptHashBlob {
    byte_count: u32,
    bytes: *mut u8,
}

#[link(name = "crypt32")]
unsafe extern "system" {
    fn CertOpenSystemStoreW(provider: usize, store_name: *const u16) -> CertificateStore;
    fn CertCloseStore(store: CertificateStore, flags: u32) -> i32;
    fn CertFindCertificateInStore(
        store: CertificateStore,
        encoding: u32,
        find_flags: u32,
        find_type: u32,
        find_parameter: *const c_void,
        previous: *const CertificateContext,
    ) -> *const CertificateContext;
    fn CertFreeCertificateContext(certificate: *const CertificateContext) -> i32;
    fn CertGetCertificateContextProperty(
        certificate: *const CertificateContext,
        property: u32,
        data: *mut c_void,
        data_size: *mut u32,
    ) -> i32;
}

/// One exact certificate context with its current-user store still open.
pub(super) struct SelectedCertificate {
    context: CertificateContextGuard,
    store: CertificateStoreGuard,
}

impl SelectedCertificate {
    pub(super) fn find(fingerprint: [u8; 32]) -> Result<Self, WindowsSigningError> {
        let store = CertificateStoreGuard::current_user_my()?;
        let mut hash = CryptHashBlob {
            byte_count: fingerprint.len() as u32,
            bytes: fingerprint.as_ptr().cast_mut(),
        };
        // SAFETY: store is an open current-user certificate store. `hash`
        // points to the fixed 32 SHA-256 bytes for this synchronous lookup, and
        // no previous certificate context is supplied.
        let context = unsafe {
            CertFindCertificateInStore(
                store.handle,
                X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
                0,
                CERT_FIND_SHA256_HASH,
                (&mut hash as *mut CryptHashBlob).cast(),
                ptr::null(),
            )
        };
        if context.is_null() {
            return Err(WindowsSigningError::CertificateUnavailable);
        }
        Ok(Self {
            context: CertificateContextGuard(context),
            store,
        })
    }

    pub(super) fn context(&self) -> *const CertificateContext {
        self.context.0
    }

    pub(super) fn store(&self) -> CertificateStore {
        self.store.handle
    }
}

struct CertificateStoreGuard {
    handle: CertificateStore,
}

impl CertificateStoreGuard {
    fn current_user_my() -> Result<Self, WindowsSigningError> {
        let name = [u16::from(b'M'), u16::from(b'Y'), 0];
        // SAFETY: `name` is the fixed NUL-terminated system-store name. A
        // legacy provider of zero selects the current user's logical store.
        let handle = unsafe { CertOpenSystemStoreW(0, name.as_ptr()) };
        if handle.is_null() {
            Err(WindowsSigningError::CertificateStoreUnavailable)
        } else {
            Ok(Self { handle })
        }
    }
}

impl Drop for CertificateStoreGuard {
    fn drop(&mut self) {
        // SAFETY: this value owns the successful certificate-store handle and
        // closes it exactly once after any certificate context is released.
        let _ = unsafe { CertCloseStore(self.handle, 0) };
    }
}

pub(super) struct CertificateContextGuard(*const CertificateContext);

impl CertificateContextGuard {
    /// Creates a guard for one certificate context returned to the caller.
    ///
    /// The context must be owned by the caller under CryptoAPI's documented
    /// `CertFreeCertificateContext` rule and must not be null.
    pub(super) unsafe fn from_owned(context: *const CertificateContext) -> Option<Self> {
        (!context.is_null()).then_some(Self(context))
    }

    pub(super) fn fingerprint(&self) -> Result<[u8; 32], WindowsSigningError> {
        let mut size = 0;
        // SAFETY: this guard owns a non-null certificate context, and the null
        // buffer asks CryptoAPI for only the fixed property size.
        if unsafe {
            CertGetCertificateContextProperty(
                self.0,
                CERT_SHA256_HASH_PROP_ID,
                std::ptr::null_mut(),
                &mut size,
            )
        } == 0
            || size != 32
        {
            return Err(WindowsSigningError::MessageFingerprintUnavailable);
        }
        let mut fingerprint = [0_u8; 32];
        // SAFETY: the fixed array has exactly the size CryptoAPI reported, and
        // the owned certificate context remains live for this synchronous call.
        if unsafe {
            CertGetCertificateContextProperty(
                self.0,
                CERT_SHA256_HASH_PROP_ID,
                fingerprint.as_mut_ptr().cast(),
                &mut size,
            )
        } == 0
        {
            return Err(WindowsSigningError::MessageFingerprintUnavailable);
        }
        Ok(fingerprint)
    }
}

impl Drop for CertificateContextGuard {
    fn drop(&mut self) {
        // SAFETY: this context came from the still-open owned store and is
        // released before that store because of SelectedCertificate field order.
        let _ = unsafe { CertFreeCertificateContext(self.0) };
    }
}
