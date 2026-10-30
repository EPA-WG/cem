//! Structural capsule bounds share the native binary shape checker.
use super::*;
pub(super) fn preflight(bytes: &[u8], control: &Work<'_>) -> Result<(), String> {
    crate::native_binary::preflight_messagepack(bytes, MAX_RECORDS, 64, || check(control))
        .map_err(|_| INVALID.into())
}
