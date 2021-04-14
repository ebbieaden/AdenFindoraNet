use crate::rust::*;
use std::ffi::CStr;
use std::os::raw::c_char;
use zei::xfr::structs::ASSET_TYPE_LENGTH;

#[no_mangle]
pub extern "C" fn rs_build_id() -> *mut c_char {
    string_to_c_char(build_id())
}

#[no_mangle]
pub extern "C" fn rs_asset_type_from_value(code: *const c_char) -> *mut c_char {
    let mut dst = [0u8; ASSET_TYPE_LENGTH];
    let c_str = unsafe { CStr::from_ptr(code) };
    dst.copy_from_slice(c_str.to_bytes());
    string_to_c_char(asset_type_from_value(dst))
}

#[no_mangle]
pub extern "C" fn rs_verify_authenticated_txn(
    state_commitment: *const c_char,
    authenticated_txn: *const c_char,
) -> bool {
    let state_commitment = c_char_to_string(state_commitment);
    let authenticated_txn = c_char_to_string(authenticated_txn);
    raw_verify_authenticated_txn(state_commitment, authenticated_txn).unwrap_or(false)
}

// TODO
#[no_mangle]
pub extern "C" fn authenticated_kv_lookup_new() -> *mut AuthenticatedKVLookup {
    unimplemented!()
    // let val = AuthenticatedKVLookup{
    // };
    //
    // let boxed_data = Box::new(val);
    // Box::into_raw(boxed_data)
}

#[no_mangle]
pub unsafe extern "C" fn authenticated_kv_lookup_destroy(
    data: *mut AuthenticatedKVLookup,
) {
    let _ = Box::from_raw(data);
}

#[no_mangle]
pub unsafe extern "C" fn rs_verify_authenticated_custom_data_result(
    state_commitment: *const c_char,
    authenticated_res: *const AuthenticatedKVLookup,
) -> bool {
    let state_commitment = c_char_to_string(state_commitment);
    raw_verify_authenticated_custom_data_result(
        state_commitment,
        &(*authenticated_res).value,
    )
    .unwrap_or(false)
}
