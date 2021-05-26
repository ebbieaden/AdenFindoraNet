use crate::rust::TransactionBuilder;

pub extern "C" fn findora_ffi_transaction_builder_add_fee_relative_auto() {}

pub extern "C" fn findora_ffi_transaction_builder_add_fee() {}

pub extern "C" fn findora_ffi_transaction_builder_check_fee() {}

pub extern "C" fn findora_ffi_transaction_builder_new() -> *mut TransactionBuilder {
    unimplemented!()
}

pub extern "C" fn findora_ffi_transaction_builder_add_operation_create_asset() {}

pub extern "C" fn findora_ffi_transaction_builder_add_operation_create_asset_with_policy()
 {
}

pub extern "C" fn findora_ffi_transaction_builder_add_policy_option() {}

pub extern "C" fn findora_ffi_transaction_builder_add_basic_issue_asset() {}

pub extern "C" fn findora_ffi_transaction_builder_add_operation_air_assign() {}

pub extern "C" fn findora_ffi_transaction_builder_add_operation_kv_update_no_hash() {}

pub extern "C" fn findora_ffi_transaction_builder_add_operation_kv_update_with_hash() {}

pub extern "C" fn findora_ffi_transaction_builder_add_operation_update_memo() {}

pub extern "C" fn findora_ffi_transaction_builder_add_transfer_operation() {}

pub extern "C" fn findora_ffi_transaction_builder_sign() {}

pub extern "C" fn findora_ffi_transaction_builder_transaction() {}

pub extern "C" fn findora_ffi_transaction_builder_transaction_handle() {}
pub extern "C" fn findora_ffi_transaction_builder_get_owner_record() {}
pub extern "C" fn findora_ffi_transaction_builder_get_owner_memo() {}
