use ledger::data_model::{AuthenticatedKVLookup, AuthenticatedTransaction};
use serde_json::Result;
use utils::HashOf;

/// Given a serialized state commitment and transaction, returns true if the transaction correctly
/// hashes up to the state commitment and false otherwise.
/// @param {string} state_commitment - String representing the state commitment.
/// @param {string} authenticated_txn - String representing the transaction.
/// @see {@link module:Network~Network#getTxn|Network.getTxn} for instructions on fetching a transaction from the ledger.
/// @see {@link module:Network~Network#getStateCommitment|Network.getStateCommitment}
/// for instructions on fetching a ledger state commitment.
/// @throws Will throw an error if the state commitment or the transaction fails to deserialize.
pub fn raw_verify_authenticated_txn(
    state_commitment: String,
    authenticated_txn: String,
) -> Result<bool> {
    let authenticated_txn =
        serde_json::from_str::<AuthenticatedTransaction>(&authenticated_txn)?;
    let state_commitment = serde_json::from_str::<HashOf<_>>(&state_commitment)?;
    Ok(authenticated_txn.is_valid(state_commitment))
}

/// Given a serialized state commitment and an authenticated custom data result, returns true if the custom data result correctly
/// hashes up to the state commitment and false otherwise.
/// @param {string} state_commitment - String representing the state commitment.
/// @param {JsValue} authenticated_txn - JSON-encoded value representing the authenticated custom
/// data result.
/// @throws Will throw an error if the state commitment or the authenticated result fail to deserialize.
pub fn raw_verify_authenticated_custom_data_result(
    state_commitment: String,
    authenticated_res: &AuthenticatedKVLookup,
) -> Result<bool> {
    let state_commitment = serde_json::from_str::<HashOf<_>>(&state_commitment)?;
    Ok(authenticated_res.is_valid(state_commitment))
}
