pub mod fee_inputs;
pub mod free;
pub mod tx_builder;

use crate::rust::types;

use crate::rust::*;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;
use zei::xfr::structs::ASSET_TYPE_LENGTH;

#[no_mangle]
pub extern "C" fn ffi_build_id() -> *mut c_char {
    string_to_c_char(build_id())
}

#[no_mangle]
pub extern "C" fn ffi_random_asset_type() -> *mut c_char {
    string_to_c_char(random_asset_type())
}

#[no_mangle]
pub extern "C" fn ffi_asset_type_from_value(code: *const c_char) -> *mut c_char {
    let mut dst = [0u8; ASSET_TYPE_LENGTH];
    let c_str = unsafe { CStr::from_ptr(code) };
    dst.copy_from_slice(c_str.to_bytes());
    string_to_c_char(rs_asset_type_from_value(dst))
}

#[no_mangle]
pub extern "C" fn ffi_verify_authenticated_txn(
    state_commitment: *const c_char,
    authenticated_txn: *const c_char,
) -> bool {
    let state_commitment = c_char_to_string(state_commitment);
    let authenticated_txn = c_char_to_string(authenticated_txn);
    rs_verify_authenticated_txn(state_commitment, authenticated_txn).unwrap_or(false)
}

// TODO
#[no_mangle]
pub extern "C" fn ffi_authenticated_kv_lookup_new() -> *mut types::AuthenticatedKVLookup
{
    unimplemented!()
    // let val = AuthenticatedKVLookup{
    // };
    //
    // let boxed_data = Box::new(val);
    // Box::into_raw(boxed_data)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_verify_authenticated_custom_data_result(
    state_commitment: *const c_char,
    authenticated_res: *const types::AuthenticatedKVLookup,
) -> bool {
    let state_commitment = c_char_to_string(state_commitment);
    rs_verify_authenticated_custom_data_result(state_commitment, &*authenticated_res)
        .unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn ffi_calculate_fee(
    ir_numerator: u64,
    ir_denominator: u64,
    outstanding_balance: u64,
) -> u64 {
    calculate_fee(ir_numerator, ir_denominator, outstanding_balance)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_get_null_pk() -> *mut types::XfrPublicKey {
    let pk = get_null_pk().into();
    Box::into_raw(Box::new(pk))
}

#[no_mangle]
pub unsafe extern "C" fn ffi_create_default_policy_info() -> *mut c_char {
    string_to_c_char(create_default_policy_info())
}

#[no_mangle]
pub unsafe extern "C" fn ffi_create_debt_policy_info(
    ir_numerator: u64,
    ir_denominator: u64,
    fiat_code: *const c_char,
    loan_amount: u64,
) -> *mut c_char {
    assert!(!fiat_code.is_null());

    if let Ok(info) = rs_create_debt_policy_info(
        ir_numerator,
        ir_denominator,
        c_char_to_string(fiat_code),
        loan_amount,
    ) {
        string_to_c_char(info)
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn ffi_create_debt_memo(
    ir_numerator: u64,
    ir_denominator: u64,
    fiat_code: *const c_char,
    loan_amount: u64,
) -> *mut c_char {
    assert!(!fiat_code.is_null());

    if let Ok(info) = rs_create_debt_memo(
        ir_numerator,
        ir_denominator,
        c_char_to_string(fiat_code),
        loan_amount,
    ) {
        string_to_c_char(info)
    } else {
        ptr::null_mut()
    }
}
