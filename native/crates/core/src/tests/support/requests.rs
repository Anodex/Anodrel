//! Canonical protocol-request fixtures for core tests.

pub(crate) fn request(operation: &str, payload: &str) -> String {
    request_at(0, operation, payload)
}

pub(crate) fn request_v1_1(operation: &str, payload: &str) -> String {
    request_at(1, operation, payload)
}

pub(crate) fn request_v1_2(operation: &str, payload: &str) -> String {
    request_at(2, operation, payload)
}

pub(crate) fn request_v1_3(operation: &str, payload: &str) -> String {
    request_at(3, operation, payload)
}

pub(crate) fn request_v1_4(operation: &str, payload: &str) -> String {
    request_at(4, operation, payload)
}

pub(crate) fn request_v1_5(operation: &str, payload: &str) -> String {
    request_at(5, operation, payload)
}

pub(crate) fn request_v1_6(operation: &str, payload: &str) -> String {
    request_at(6, operation, payload)
}

pub(crate) fn request_v1_7(operation: &str, payload: &str) -> String {
    request_at(7, operation, payload)
}

pub(crate) fn request_v1_8(operation: &str, payload: &str) -> String {
    request_at(8, operation, payload)
}

pub(crate) fn request_v1_9(operation: &str, payload: &str) -> String {
    request_at(9, operation, payload)
}

pub(crate) fn request_v1_10(operation: &str, payload: &str) -> String {
    request_at(10, operation, payload)
}

pub(crate) fn request_v1_12(operation: &str, payload: &str) -> String {
    request_at(12, operation, payload)
}

pub(crate) fn request_v1_13(operation: &str, payload: &str) -> String {
    request_at(13, operation, payload)
}

pub(crate) fn request_v1_15(operation: &str, payload: &str) -> String {
    request_at(15, operation, payload)
}

pub(crate) fn request_v1_28(operation: &str, payload: &str) -> String {
    request_at(28, operation, payload)
}

pub(crate) fn request_v1_29(operation: &str, payload: &str) -> String {
    request_at(29, operation, payload)
}

fn request_at(minor: u16, operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":{minor}}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}
