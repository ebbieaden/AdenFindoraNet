use crate::rust::transaction::FeeInputs;
use crate::rust::types::*;

pub extern "C" fn findora_ffi_fee_inputs_new() -> *mut FeeInputs {
    Box::into_raw(Box::new(FeeInputs::new()))
}

#[no_mangle]
pub unsafe extern "C" fn findora_ffi_fee_inputs_free(ptr: *mut FeeInputs) {
    if ptr.is_null() {
        return;
    }
    Box::from_raw(ptr);
}

pub unsafe extern "C" fn findora_ffi_fee_inputs_append(
    ptr: *mut FeeInputs,
    am: u64,
    tr: *const TxoRef,
    ar: *const ClientAssetRecord,
    om: *const OwnerMemo,
    kp: *const XfrKeyPair,
) {
    assert!(!ptr.is_null());
    let input = &mut *ptr;

    let om_op;
    if om.is_null() {
        om_op = None;
    } else {
        om_op = Some((**om).clone());
    }

    input.append(am, **tr, (**ar).clone(), om_op, (**kp).clone());
}
