use crate::rust::types;
use crate::rust::*;

#[no_mangle]
/// Fee smaller than this value will be denied.
pub extern "C" fn findora_ffi_fra_get_minimal_fee() -> u64 {
    fra_get_minimal_fee()
}

#[no_mangle]
/// The destination for fee to be transfered to.
pub extern "C" fn findora_ffi_fra_get_dest_pubkey() -> *mut types::XfrPublicKey {
    Box::into_raw(Box::new(types::XfrPublicKey::from(fra_get_dest_pubkey())))
}

#[no_mangle]
pub extern "C" fn findora_ffi_fee_inputs_new() -> *mut FeeInputs {
    Box::into_raw(Box::new(FeeInputs::new()))
}

#[no_mangle]
pub unsafe extern "C" fn findora_ffi_fee_inputs_append(
    ptr: *mut FeeInputs,
    am: u64,
    tr: *const TxoRef,
    ar: *const ClientAssetRecord,
    om: *const OwnerMemo,
    kp: *const types::XfrKeyPair,
) {
    assert!(!ptr.is_null());
    let input = &mut *ptr;

    let om_op;
    if om.is_null() {
        om_op = None;
    } else {
        om_op = Some((*om).clone());
    }

    input.append(am, *tr, (*ar).clone(), om_op, (**kp).clone());
}
