//! Attached CMS message signing and exact-publisher verification.

use std::{
    ffi::{c_char, c_void},
    mem, ptr,
};

use crate::{
    WindowsSigningError,
    certificate::{CertificateContext, CertificateContextGuard, SelectedCertificate},
};

type Bool = i32;

const X509_ASN_ENCODING: u32 = 0x0000_0001;
const PKCS_7_ASN_ENCODING: u32 = 0x0001_0000;
const CRYPT_MESSAGE_SILENT_KEYSET_FLAG: u32 = 0x0000_0040;
const SHA_256_OID: &[u8] = b"2.16.840.1.101.3.4.2.1\0";

#[repr(C)]
struct CryptDataBlob {
    byte_count: u32,
    bytes: *mut u8,
}

#[repr(C)]
struct CryptAlgorithmIdentifier {
    object_identifier: *mut c_char,
    parameters: CryptDataBlob,
}

#[repr(C)]
struct CryptSignMessagePara {
    size: u32,
    encoding: u32,
    signing_certificate: *const CertificateContext,
    hash_algorithm: CryptAlgorithmIdentifier,
    hash_auxiliary: *mut c_void,
    certificate_count: u32,
    certificates: *const *const CertificateContext,
    crl_count: u32,
    crls: *mut c_void,
    authenticated_attribute_count: u32,
    authenticated_attributes: *mut c_void,
    unauthenticated_attribute_count: u32,
    unauthenticated_attributes: *mut c_void,
    flags: u32,
    inner_content_type: u32,
}

#[repr(C)]
struct CryptVerifyMessagePara {
    size: u32,
    encoding: u32,
    cryptographic_provider: usize,
    signer_certificate_callback: *mut c_void,
    callback_argument: *mut c_void,
}

#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptSignMessage(
        parameters: *const CryptSignMessagePara,
        detached_signature: Bool,
        message_count: u32,
        message_parts: *const *const u8,
        message_lengths: *const u32,
        signed_blob: *mut u8,
        signed_blob_length: *mut u32,
    ) -> Bool;
    fn CryptGetMessageSignerCount(
        encoding: u32,
        signed_blob: *const u8,
        signed_blob_length: u32,
    ) -> i32;
    fn CryptVerifyMessageSignature(
        parameters: *const CryptVerifyMessagePara,
        signer_index: u32,
        signed_blob: *const u8,
        signed_blob_length: u32,
        decoded_message: *mut u8,
        decoded_message_length: *mut u32,
        signer_certificate: *mut *const CertificateContext,
    ) -> Bool;
}

/// Signs one bounded message into an attached CMS envelope.
pub(super) fn sign_attached(
    message: &[u8],
    maximum_output_bytes: usize,
    fingerprint: [u8; 32],
) -> Result<Vec<u8>, WindowsSigningError> {
    let message_length = checked_dword(message.len())?;
    let maximum_output = checked_dword(maximum_output_bytes)?;
    let certificate = SelectedCertificate::find(fingerprint)?;
    let certificates = [certificate.context()];
    let parameters = CryptSignMessagePara {
        size: size_of::<CryptSignMessagePara>(),
        encoding: X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
        signing_certificate: certificate.context(),
        hash_algorithm: sha256_algorithm(),
        hash_auxiliary: ptr::null_mut(),
        certificate_count: certificates.len() as u32,
        certificates: certificates.as_ptr(),
        crl_count: 0,
        crls: ptr::null_mut(),
        authenticated_attribute_count: 0,
        authenticated_attributes: ptr::null_mut(),
        unauthenticated_attribute_count: 0,
        unauthenticated_attributes: ptr::null_mut(),
        flags: CRYPT_MESSAGE_SILENT_KEYSET_FLAG,
        inner_content_type: 0,
    };
    let message_parts = [message.as_ptr()];
    let message_lengths = [message_length];
    let mut signed = vec![0_u8; maximum_output_bytes];
    let mut signed_length = maximum_output;
    // SAFETY: the parameters use the documented C layouts, all pointer-backed
    // values remain live throughout the synchronous call, one current-user
    // certificate is included with the attached message, and the output buffer
    // capacity is exactly the caller-selected bounded length.
    let success = unsafe {
        CryptSignMessage(
            &parameters,
            0,
            1,
            message_parts.as_ptr(),
            message_lengths.as_ptr(),
            signed.as_mut_ptr(),
            &mut signed_length,
        )
    };
    if success == 0 || signed_length as usize > signed.len() {
        return Err(WindowsSigningError::MessageSigningFailed);
    }
    signed.truncate(signed_length as usize);
    Ok(signed)
}

/// Verifies one exact single-signer attached CMS envelope.
pub(super) fn verify_attached(
    signed_message: &[u8],
    maximum_decoded_bytes: usize,
    expected_fingerprint: [u8; 32],
) -> Result<Vec<u8>, WindowsSigningError> {
    let signed_length = checked_dword(signed_message.len())?;
    let maximum_decoded = checked_dword(maximum_decoded_bytes)?;
    // SAFETY: the bounded slice stays live through this synchronous CryptoAPI
    // message inspection, and the fixed encoding selects standard X.509/CMS.
    let signer_count = unsafe {
        CryptGetMessageSignerCount(
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            signed_message.as_ptr(),
            signed_length,
        )
    };
    if signer_count != 1 {
        return Err(WindowsSigningError::MessageVerificationFailed);
    }
    let parameters = CryptVerifyMessagePara {
        size: size_of::<CryptVerifyMessagePara>(),
        encoding: X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
        cryptographic_provider: 0,
        signer_certificate_callback: ptr::null_mut(),
        callback_argument: ptr::null_mut(),
    };
    let mut decoded = vec![0_u8; maximum_decoded_bytes];
    let mut decoded_length = maximum_decoded;
    let mut raw_signer = ptr::null();
    // SAFETY: the parameters use the documented C layout, the signed input and
    // decoded output remain live through the synchronous call, index zero is
    // the only signer after the count check, and `raw_signer` receives one
    // context owned by the caller when verification succeeds.
    let success = unsafe {
        CryptVerifyMessageSignature(
            &parameters,
            0,
            signed_message.as_ptr(),
            signed_length,
            decoded.as_mut_ptr(),
            &mut decoded_length,
            &mut raw_signer,
        )
    };
    // SAFETY: CryptoAPI documents `raw_signer` as caller-owned when supplied.
    // The guard is created before any early return so an unexpected failure
    // result cannot leak a returned context.
    let signer = unsafe { CertificateContextGuard::from_owned(raw_signer) };
    if success == 0 || decoded_length as usize > decoded.len() {
        return Err(WindowsSigningError::MessageVerificationFailed);
    }
    let signer = signer.ok_or(WindowsSigningError::MessageSignerUnavailable)?;
    if signer.fingerprint()? != expected_fingerprint {
        return Err(WindowsSigningError::MessagePublisherMismatch);
    }
    decoded.truncate(decoded_length as usize);
    Ok(decoded)
}

fn sha256_algorithm() -> CryptAlgorithmIdentifier {
    CryptAlgorithmIdentifier {
        object_identifier: SHA_256_OID.as_ptr().cast_mut().cast(),
        parameters: CryptDataBlob {
            byte_count: 0,
            bytes: ptr::null_mut(),
        },
    }
}

fn checked_dword(value: usize) -> Result<u32, WindowsSigningError> {
    u32::try_from(value)
        .ok()
        .filter(|value| *value != 0)
        .ok_or(WindowsSigningError::MessageLimitInvalid)
}

const fn size_of<T>() -> u32 {
    mem::size_of::<T>() as u32
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::{CryptSignMessagePara, CryptVerifyMessagePara};

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn message_structures_match_the_default_64_bit_windows_sdk_layouts() {
        assert_eq!(mem::size_of::<CryptSignMessagePara>(), 120);
        assert_eq!(mem::size_of::<CryptVerifyMessagePara>(), 32);
    }
}
