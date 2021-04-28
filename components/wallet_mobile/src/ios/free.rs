use crate::rust::types::{AuthenticatedKVLookup, XfrPublicKey};

#[no_mangle]
pub unsafe extern "C" fn findora_ffi_authenticated_kv_lookup_free(
    ptr: *mut AuthenticatedKVLookup,
) {
    if ptr.is_null() {
        return;
    }
    Box::from_raw(ptr);
}

#[no_mangle]
pub unsafe extern "C" fn findora_ffi_xfr_public_key_free(ptr: *mut XfrPublicKey) {
    if ptr.is_null() {
        return;
    }
    Box::from_raw(ptr);
}
