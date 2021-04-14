use crate::rust::*;
use core::fmt::Display;
use ledger::data_model::AuthenticatedKVLookup;
use ruc::{d, err::RucResult};
use wasm_bindgen::prelude::*;
use zei::xfr::structs::ASSET_TYPE_LENGTH;

#[inline(always)]
fn error_to_jsvalue<T: Display>(e: T) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen]
/// Generates asset type as a Base64 string from a JSON-serialized JavaScript value.
pub fn asset_type_from_jsvalue(val: &JsValue) -> Result<String, JsValue> {
    let code: [u8; ASSET_TYPE_LENGTH] =
        val.into_serde().c(d!()).map_err(error_to_jsvalue)?;
    Ok(asset_type_from_value(code))
}

#[wasm_bindgen]
/// Given a serialized state commitment and transaction, returns true if the transaction correctly
/// hashes up to the state commitment and false otherwise.
/// @param {string} state_commitment - String representing the state commitment.
/// @param {string} authenticated_txn - String representing the transaction.
/// @see {@link module:Network~Network#getTxn|Network.getTxn} for instructions on fetching a transaction from the ledger.
/// @see {@link module:Network~Network#getStateCommitment|Network.getStateCommitment}
/// for instructions on fetching a ledger state commitment.
/// @throws Will throw an error if the state commitment or the transaction fails to deserialize.
pub fn verify_authenticated_txn(
    state_commitment: String,
    authenticated_txn: String,
) -> Result<bool, JsValue> {
    let is_valid = raw_verify_authenticated_txn(state_commitment, authenticated_txn)
        .c(d!())
        .map_err(error_to_jsvalue)?;
    Ok(is_valid)
}

#[wasm_bindgen]
/// Given a serialized state commitment and an authenticated custom data result, returns true if the custom data result correctly
/// hashes up to the state commitment and false otherwise.
/// @param {string} state_commitment - String representing the state commitment.
/// @param {JsValue} authenticated_txn - JSON-encoded value representing the authenticated custom
/// data result.
/// @throws Will throw an error if the state commitment or the authenticated result fail to deserialize.
pub fn verify_authenticated_custom_data_result(
    state_commitment: String,
    authenticated_res: JsValue,
) -> Result<bool, JsValue> {
    let authenticated_res: AuthenticatedKVLookup =
        authenticated_res.into_serde().c(d!()).map_err(|e| {
            JsValue::from_str(&format!(
                "couldn't deserialize the authenticated custom data lookup: {}",
                e
            ))
        })?;
    let is_valid = raw_verify_authenticated_custom_data_result(
        state_commitment,
        &authenticated_res,
    )
    .c(d!())
    .map_err(|e| {
        JsValue::from_str(&format!("Could not deserialize state commitment: {}", e))
    })?;

    Ok(is_valid)
}
