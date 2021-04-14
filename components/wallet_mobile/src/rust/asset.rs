use super::util::*;
use ledger::data_model::AssetTypeCode;
use std::os::raw::c_char;
use wasm_bindgen::prelude::*;
use zei::xfr::structs::{AssetType as ZeiAssetType, ASSET_TYPE_LENGTH};

#[wasm_bindgen]
/// Generates random Base64 encoded asset type as a Base64 string. Used in asset definitions.
/// @see {@link
/// module:Findora-Wasm~TransactionBuilder#add_operation_create_asset|add_operation_create_asset}
/// for instructions on how to define an asset with a new
/// asset type
pub fn random_asset_type() -> String {
    AssetTypeCode::gen_random().to_base64()
}

#[no_mangle]
pub extern "C" fn rs_random_asset_type() -> *mut c_char {
    string_to_c_char(AssetTypeCode::gen_random().to_base64())
}

/// Generates asset type as a Base64 string from given code.
pub fn asset_type_from_value(code: [u8; ASSET_TYPE_LENGTH]) -> String {
    AssetTypeCode {
        val: ZeiAssetType(code),
    }
    .to_base64()
}
