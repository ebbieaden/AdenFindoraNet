use crate::rust::*;
use credentials::{CredIssuerPublicKey, CredUserPublicKey};
use std::os::raw::c_char;
use zei::xfr::sig::XfrKeyPair;

#[no_mangle]
/// @param am: amount to pay
/// @param kp: owner's XfrKeyPair
pub extern "C" fn findora_ffi_transaction_builder_add_fee_relative_auto(
    builder: &TransactionBuilder,
    am: u64,
    kp: &XfrKeyPair,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_fee_relative_auto(am, kp.clone()) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Use this func to get the necessary infomations for generating `Relative Inputs`
///
/// - TxoRef::Relative("Element index of the result")
/// - ClientAssetRecord::from_json("Element of the result")
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_get_relative_outputs(
    builder: &TransactionBuilder,
) -> safer_ffi::vec::Vec<ClientAssetRecord> {
    builder.clone().get_relative_outputs().into()
}

/// As the last operation of any transaction,
/// add a static fee to the transaction.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_fee(
    builder: &TransactionBuilder,
    inputs: &FeeInputs,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_fee(inputs.clone()) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// A simple fee checker for mainnet v1.0.
///
/// SEE [check_fee](ledger::data_model::Transaction::check_fee)
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_check_fee(
    builder: &TransactionBuilder,
) -> bool {
    builder.clone().check_fee()
}

/// Create a new transaction builder.
/// @param {BigInt} seq_id - Unique sequence ID to prevent replay attacks.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_new(
    seq_id: u64,
) -> *mut TransactionBuilder {
    Box::into_raw(Box::new(TransactionBuilder::new(seq_id)))
}

/// Wraps around TransactionBuilder to add an asset definition operation to a transaction builder instance.
/// @example <caption> Error handling </caption>
/// try {
///     await wasm.add_operation_create_asset(wasm.new_keypair(), "test_memo", wasm.random_asset_type(), wasm.AssetRules.default());
/// } catch (err) {
///     console.log(err)
/// }
///
/// @param {XfrKeyPair} key_pair -  Issuer XfrKeyPair.
/// @param {string} memo - Text field for asset definition.
/// @param {string} token_code - Optional Base64 string representing the token code of the asset to be issued.
/// If empty, a token code will be chosen at random.
/// @param {AssetRules} asset_rules - Asset rules object specifying which simple policies apply
/// to the asset.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_create_asset(
    builder: &TransactionBuilder,
    key_pair: &XfrKeyPair,
    memo: *const c_char,
    token_code: *const c_char,
    asset_rules: &AssetRules,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_operation_create_asset(
        key_pair,
        c_char_to_string(memo),
        c_char_to_string(token_code),
        asset_rules.clone(),
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Wraps around TransactionBuilder to add an asset issuance to a transaction builder instance.
///
/// Use this function for simple one-shot issuances.
///
/// @param {XfrKeyPair} key_pair  - Issuer XfrKeyPair.
/// and types of traced assets.
/// @param {string} code - base64 string representing the token code of the asset to be issued.
/// @param {BigInt} seq_num - Issuance sequence number. Every subsequent issuance of a given asset type must have a higher sequence number than before.
/// @param {BigInt} amount - Amount to be issued.
/// @param {boolean} conf_amount - `true` means the asset amount is confidential, and `false` means it's nonconfidential.
/// @param {PublicParams} zei_params - Public parameters necessary to generate asset records.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_basic_issue_asset(
    builder: &TransactionBuilder,
    key_pair: &XfrKeyPair,
    code: *const c_char,
    seq_num: u64,
    amount: u64,
    conf_amount: bool,
    zei_params: &PublicParams,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_basic_issue_asset(
        key_pair,
        c_char_to_string(code),
        seq_num,
        amount,
        conf_amount,
        zei_params,
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds an operation to the transaction builder that appends a credential commitment to the address
/// identity registry.
/// @param {XfrKeyPair} key_pair - Ledger key that is tied to the credential.
/// @param {CredUserPublicKey} user_public_key - Public key of the credential user.
/// @param {CredIssuerPublicKey} issuer_public_key - Public key of the credential issuer.
/// @param {CredentialCommitment} commitment - Credential commitment to add to the address identity registry.
/// @param {CredPoK} pok- Proof that the credential commitment is valid.
/// @see {@link module:Findora-Wasm.wasm_credential_commit|wasm_credential_commit} for information about how to generate a credential
/// commitment.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_air_assign(
    builder: &TransactionBuilder,
    key_pair: &XfrKeyPair,
    user_public_key: &CredUserPublicKey,
    issuer_public_key: &CredIssuerPublicKey,
    commitment: &CredentialCommitment,
    pok: &CredentialPoK,
) -> *mut TransactionBuilder {
    if let Ok(info) = (*builder).clone().add_operation_air_assign(
        key_pair,
        user_public_key,
        issuer_public_key,
        commitment,
        pok,
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds an operation to the transaction builder that removes a hash from ledger's custom data
/// store.
/// @param {XfrKeyPair} auth_key_pair - Key pair that is authorized to delete the hash at the
/// provided key.
/// @param {Key} key - The key of the custom data store whose value will be cleared if the
/// transaction validates.
/// @param {BigInt} seq_num - Nonce to prevent replays.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_kv_update_no_hash(
    builder: &TransactionBuilder,
    auth_key_pair: &XfrKeyPair,
    key: &Key,
    seq_num: u64,
) -> *mut TransactionBuilder {
    if let Ok(info) =
        builder
            .clone()
            .add_operation_kv_update_no_hash(auth_key_pair, key, seq_num)
    {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds an operation to the transaction builder that adds a hash to the ledger's custom data
/// store.
/// @param {XfrKeyPair} auth_key_pair - Key pair that is authorized to add the hash at the
/// provided key.
/// @param {Key} key - The key of the custom data store the value will be added to if the
/// transaction validates.
/// @param {KVHash} hash - The hash to add to the custom data store.
/// @param {BigInt} seq_num - Nonce to prevent replays.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_kv_update_with_hash(
    builder: &TransactionBuilder,
    auth_key_pair: &XfrKeyPair,
    key: &Key,
    seq_num: u64,
    kv_hash: &KVHash,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_operation_kv_update_with_hash(
        auth_key_pair,
        key,
        seq_num,
        kv_hash,
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds an operation to the transaction builder that adds a hash to the ledger's custom data
/// store.
/// @param {XfrKeyPair} auth_key_pair - Asset creator key pair.
/// @param {String} code - base64 string representing token code of the asset whose memo will be updated.
/// transaction validates.
/// @param {String} new_memo - The new asset memo.
/// @see {@link module:Findora-Wasm~AssetRules#set_updatable|AssetRules.set_updatable} for more information about how
/// to define an updatable asset.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_operation_update_memo(
    builder: &TransactionBuilder,
    auth_key_pair: &XfrKeyPair,
    code: *const c_char,
    new_memo: *const c_char,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_operation_update_memo(
        auth_key_pair,
        c_char_to_string(code),
        c_char_to_string(new_memo),
    ) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Adds a serialized transfer asset operation to a transaction builder instance.
/// @param {string} op - a JSON-serialized transfer operation.
/// @see {@link module:Findora-Wasm~TransferOperationBuilder} for details on constructing a transfer operation.
/// @throws Will throw an error if `op` fails to deserialize.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_add_transfer_operation(
    builder: &TransactionBuilder,
    op: *const c_char,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().add_transfer_operation(c_char_to_string(op)) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_sign(
    builder: &TransactionBuilder,
    kp: &XfrKeyPair,
) -> *mut TransactionBuilder {
    if let Ok(info) = builder.clone().sign(kp) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}

/// Extracts the serialized form of a transaction.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_transaction(
    builder: &TransactionBuilder,
) -> *mut c_char {
    string_to_c_char(builder.transaction())
}

/// Calculates transaction handle.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_transaction_handle(
    builder: &TransactionBuilder,
) -> *mut c_char {
    string_to_c_char(builder.transaction_handle())
}

/// Fetches a client record from a transaction.
/// @param {number} idx - Record to fetch. Records are added to the transaction builder sequentially.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_get_owner_record(
    builder: &TransactionBuilder,
    idx: usize,
) -> *mut ClientAssetRecord {
    Box::into_raw(Box::new(builder.get_owner_record(idx)))
}

/// Fetches an owner memo from a transaction
/// @param {number} idx - Owner memo to fetch. Owner memos are added to the transaction builder sequentially.
#[no_mangle]
pub extern "C" fn findora_ffi_transaction_builder_get_owner_memo(
    builder: &TransactionBuilder,
    idx: usize,
) -> *mut OwnerMemo {
    if let Some(info) = builder.get_owner_memo(idx) {
        Box::into_raw(Box::new(info))
    } else {
        std::ptr::null_mut()
    }
}
