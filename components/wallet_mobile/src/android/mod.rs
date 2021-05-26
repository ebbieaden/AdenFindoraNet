mod constructor;

use crate::rust::types;
use crate::rust::*;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jbyteArray, jint, jlong, jstring, jvalue, JNI_TRUE};
use jni::JNIEnv;
use ledger::data_model::AssetType as PlatformAssetType;
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey};
use zei::xfr::structs::{OwnerMemo as ZeiOwnerMemo, ASSET_TYPE_LENGTH};

#[no_mangle]
/// Returns the git commit hash and commit date of the commit this library was built against.
pub extern "system" fn Java_com_findora_JniApi_buildId(
    env: JNIEnv,
    // this is the class that owns our
    // static method. Not going to be
    // used, but still needs to have
    // an argument slot
    _: JClass,
) -> jstring {
    let output = env
        .new_string(build_id())
        .expect("Couldn't create java string!");
    // extract the raw pointer to return.
    output.into_inner()
}

#[no_mangle]
/// Generates asset type as a Base64 string from a JSON-serialized JavaScript value.
pub extern "system" fn Java_com_findora_JniApi_assetTypeFromValue(
    env: JNIEnv,
    _: JClass,
    input: jbyteArray,
) -> jstring {
    let input = env.convert_byte_array(input).unwrap();
    let mut buf = [0u8; ASSET_TYPE_LENGTH];
    buf.copy_from_slice(input.as_ref());

    let asset_type = rs_asset_type_from_value(buf);
    let output = env
        .new_string(asset_type)
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Given a serialized state commitment and transaction, returns true if the transaction correctly
/// hashes up to the state commitment and false otherwise.
/// @param {string} state_commitment - String representing the state commitment.
/// @param {string} authenticated_txn - String representing the transaction.
/// @see {@link module:Network~Network#getTxn|Network.getTxn} for instructions on fetching a transaction from the ledger.
/// @see {@link module:Network~Network#getStateCommitment|Network.getStateCommitment}
/// for instructions on fetching a ledger state commitment.
/// @throws Will throw an error if the state commitment or the transaction fails to deserialize.
pub extern "system" fn Java_com_findora_JniApi_verifyAuthenticatedTxn(
    env: JNIEnv,
    _: JClass,
    state_commitment: JString,
    authenticated_txn: JString,
) -> jboolean {
    let state_commitment: String = env
        .get_string(state_commitment)
        .expect("Couldn't get java string!")
        .into();

    let authenticated_txn: String = env
        .get_string(authenticated_txn)
        .expect("Couldn't get java string!")
        .into();

    rs_verify_authenticated_txn(state_commitment, authenticated_txn).unwrap_or(false)
        as jboolean
}

#[no_mangle]
/// Given a serialized state commitment and an authenticated custom data result, returns true if the custom data result correctly
/// hashes up to the state commitment and false otherwise.
/// @param {string} state_commitment - String representing the state commitment.
/// @param {JsValue} authenticated_txn - JSON-encoded value representing the authenticated custom
/// data result.
/// @throws Will throw an error if the state commitment or the authenticated result fail to deserialize.
pub unsafe extern "system" fn Java_com_findora_JniApi_verifyAuthenticatedCustomAataResult(
    env: JNIEnv,
    _: JClass,
    state_commitment: JString,
    authenticated_res_ptr: jlong,
) -> jboolean {
    let state_commitment: String = env
        .get_string(state_commitment)
        .expect("Couldn't get java string!")
        .into();

    let res = &mut *(authenticated_res_ptr as *mut types::AuthenticatedKVLookup);

    rs_verify_authenticated_custom_data_result(state_commitment, &res).unwrap_or(false)
        as jboolean
}

#[no_mangle]
/// Generate mnemonic with custom length and language.
/// - @param `wordslen`: acceptable value are one of [ 12, 15, 18, 21, 24 ]
/// - @param `lang`: acceptable value are one of [ "en", "zh", "zh_traditional", "fr", "it", "ko", "sp", "jp" ]
pub extern "system" fn Java_com_findora_JniApi_generateMnemonicCustom(
    env: JNIEnv,
    _: JClass,
    words_len: jint,
    lang: JString,
) -> jstring {
    let lang: String = env
        .get_string(lang)
        .expect("Couldn't get java string!")
        .into();
    let mnemonic = rs_generate_mnemonic_custom(words_len as u8, lang.as_str()).unwrap();
    let output = env
        .new_string(mnemonic)
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_findora_JniApi_decryptionPbkdf2Aes256gcm(
    env: JNIEnv,
    _: JClass,
    enc_key_pair: jbyteArray,
    password: JString,
) -> jstring {
    let enc_key_pair = env.convert_byte_array(enc_key_pair).unwrap();
    let password: String = env
        .get_string(password)
        .expect("Couldn't get java string!")
        .into();
    let plaintext = decryption_pbkdf2_aes256gcm(enc_key_pair, password);
    let output = env
        .new_string(plaintext)
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_findora_JniApi_encryptionPbkdf2Aes256gcm(
    env: JNIEnv,
    _: JClass,
    key_pair: JString,
    password: JString,
) -> jbyteArray {
    let key_pair: String = env
        .get_string(key_pair)
        .expect("Couldn't get java string!")
        .into();
    let password: String = env
        .get_string(password)
        .expect("Couldn't get java string!")
        .into();

    let res = encryption_pbkdf2_aes256gcm(key_pair, password);
    env.byte_array_from_slice(res.as_slice()).unwrap()
}

#[no_mangle]
/// Constructs a transfer key pair from a hex-encoded string.
/// The encode a key pair, use `keypair_to_str` function.
pub extern "system" fn Java_com_findora_JniApi_keypairFromStr(
    env: JNIEnv,
    _: JClass,
    text: JString,
) -> jlong {
    let text: String = env
        .get_string(text)
        .expect("Couldn't get java string!")
        .into();
    let val = types::XfrKeyPair::from(keypair_from_str(text));
    Box::into_raw(Box::new(val)) as jlong
}

#[no_mangle]
/// Returns bech32 encoded representation of an XfrPublicKey.
pub unsafe extern "system" fn Java_com_findora_JniApi_publicKeyToBech32(
    env: JNIEnv,
    _: JClass,
    xfr_public_key_ptr: jlong,
) -> jstring {
    let key = &*(xfr_public_key_ptr as *mut types::XfrPublicKey);
    let res = public_key_to_bech32(key);
    let output = env.new_string(res).expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Extracts the public key as a string from a transfer key pair.
pub unsafe extern "system" fn Java_com_findora_JniApi_getPubKeyStr(
    env: JNIEnv,
    _: JClass,
    xfr_keypair_ptr: jlong,
) -> jstring {
    let key = &*(xfr_keypair_ptr as *mut types::XfrKeyPair);
    let pubkey = get_pub_key_str(key);
    let output = env
        .new_string(pubkey)
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Extracts the private key as a string from a transfer key pair.
pub unsafe extern "system" fn Java_com_findora_JniApi_getPrivKeyStr(
    env: JNIEnv,
    _: JClass,
    xfr_keypair_ptr: jlong,
) -> jstring {
    let key = &*(xfr_keypair_ptr as *mut types::XfrKeyPair);
    let prikey = get_priv_key_str(key);
    let output = env
        .new_string(prikey)
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Restore the XfrKeyPair from a mnemonic with a default bip44-path,
/// that is "m/44'/917'/0'/0/0" ("m/44'/coin'/account'/change/address").
pub extern "system" fn Java_com_findora_JniApi_restoreKeypairFromMnemonicDefault(
    env: JNIEnv,
    _: JClass,
    phrase: JString,
) -> jlong {
    let phrase: String = env
        .get_string(phrase)
        .expect("Couldn't get java string!")
        .into();
    if let Ok(keypair) = rs_restore_keypair_from_mnemonic_default(phrase.as_str()) {
        Box::into_raw(Box::new(types::XfrKeyPair::from(keypair))) as jlong
    } else {
        ::std::ptr::null_mut::<()>() as jlong
    }
}

#[no_mangle]
/// Expresses a transfer key pair as a hex-encoded string.
/// To decode the string, use `keypair_from_str` function.
pub unsafe extern "system" fn Java_com_findora_JniApi_keypairToStr(
    env: JNIEnv,
    _: JClass,
    xfr_keypair_ptr: jlong,
) -> jstring {
    let key = &*(xfr_keypair_ptr as *mut types::XfrKeyPair);
    let res = keypair_to_str(key);
    let output = env.new_string(res).expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_findora_JniApi_createKeypairFromSecret(
    env: JNIEnv,
    _: JClass,
    sk_str: JString,
) -> jlong {
    let sk: String = env
        .get_string(sk_str)
        .expect("Couldn't get java string!")
        .into();
    if let Some(keypair) = create_keypair_from_secret(sk) {
        Box::into_raw(Box::new(types::XfrKeyPair::from(keypair))) as jlong
    } else {
        ::std::ptr::null_mut::<()>() as jlong
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_findora_JniApi_getPkFromKeypair(
    _env: JNIEnv,
    _: JClass,
    xfr_keypair_ptr: jlong,
) -> jlong {
    let kp = &*(xfr_keypair_ptr as *mut types::XfrKeyPair);
    let pk = get_pk_from_keypair(kp);
    Box::into_raw(Box::new(types::XfrPublicKey::from(pk))) as jlong
}

#[no_mangle]
/// Creates a new transfer key pair.
pub extern "system" fn Java_com_findora_JniApi_newKeypair(
    _env: JNIEnv,
    _: JClass,
) -> jlong {
    let keypair = new_keypair();
    Box::into_raw(Box::new(types::XfrKeyPair::from(keypair))) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_findora_JniApi_bech32ToBase64(
    env: JNIEnv,
    _: JClass,
    pk: JString,
) -> jstring {
    let pk: String = env
        .get_string(pk)
        .expect("Couldn't get java string!")
        .into();

    let bs = rs_bech32_to_base64(pk.as_str()).unwrap();
    let output = env.new_string(bs).expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
pub extern "system" fn Java_com_findora_JniApi_base64ToBech32(
    env: JNIEnv,
    _: JClass,
    pk: JString,
) -> jstring {
    let pk: String = env
        .get_string(pk)
        .expect("Couldn't get java string!")
        .into();

    let bs = rs_base64_to_bech32(pk.as_str()).unwrap();
    let output = env.new_string(bs).expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Builds an asset type from a JSON-encoded JavaScript value.
/// @param {JsValue} val - JSON-encoded asset type fetched from ledger server with the `asset_token/{code}` route.
/// Note: The first field of an asset type is `properties`. See the example below.
///
/// @example
/// "properties":{
///   "code":{
///     "val":[151,8,106,38,126,101,250,236,134,77,83,180,43,152,47,57,83,30,60,8,132,218,48,52,167,167,190,244,34,45,78,80]
///   },
///   "issuer":{"key":“iFW4jY_DQVSGED05kTseBBn0BllPB9Q9escOJUpf4DY=”},
///   "memo":“test memo”,
///   "asset_rules":{
///     "transferable":true,
///     "updatable":false,
///     "transfer_multisig_rules":null,
///     "max_units":5000
///   }
/// }
///
/// @see {@link module:Findora-Network~Network#getAssetProperties|Network.getAsset} for information about how to
/// fetch an asset type from the ledger server.
pub unsafe extern "system" fn Java_com_findora_JniApi_assetTypeFromJson(
    env: JNIEnv,
    _: JClass,
    asset_type_json: JString,
) -> jlong {
    let asset_type_json: String = env
        .get_string(asset_type_json)
        .expect("Couldn't get java string!")
        .into();

    let asset_type: PlatformAssetType =
        serde_json::from_str(asset_type_json.as_str()).unwrap();
    Box::into_raw(Box::new(AssetType::from_json(asset_type).unwrap())) as jlong
}

#[no_mangle]
/// Fetch the tracing policies associated with this asset type.
/// @returns {TracingPolicies}
pub unsafe extern "system" fn Java_com_findora_JniApi_assetTypeGetTracingPolicies(
    _env: JNIEnv,
    _: JClass,
    asset_type: jlong,
) -> jlong {
    let asset_type = &*(asset_type as *mut AssetType);
    let policy = asset_type.get_tracing_policies();
    Box::into_raw(Box::new(policy)) as jlong
}

#[no_mangle]
/// Converts a base64 encoded public key string to a public key.
/// @param {string} pk
/// @returns {XfrPublicKey}
pub unsafe extern "system" fn Java_com_findora_JniApi_publicKeyFromBase64(
    env: JNIEnv,
    _: JClass,
    pk: JString,
) -> jlong {
    let pk: String = env
        .get_string(pk)
        .expect("Couldn't get java string!")
        .into();

    let key = rs_public_key_from_base64(pk.as_str()).unwrap();
    Box::into_raw(Box::new(key)) as jlong
}

#[no_mangle]
/// Creates a relative txo reference as a JSON string. Relative txo references are offset
/// backwards from the operation they appear in -- 0 is the most recent, (n-1) is the first output
/// of the transaction.
///
/// Use relative txo indexing when referring to outputs of intermediate operations (e.g. a
/// transaction containing both an issuance and a transfer).
///
/// # Arguments
/// @param {BigInt} idx -  Relative TXO (transaction output) SID.
/// @returns {TxoRef}
pub unsafe extern "system" fn Java_com_findora_JniApi_txoRefRelative(
    _env: JNIEnv,
    _: JClass,
    idx: jint,
) -> jlong {
    Box::into_raw(Box::new(TxoRef::relative(idx as u64))) as jlong
}

#[no_mangle]
/// Creates an absolute transaction reference as a JSON string.
///
/// Use absolute txo indexing when referring to an output that has been assigned a utxo index (i.e.
/// when the utxo has been committed to the ledger in an earlier transaction).
///
/// # Arguments
/// @param {BigInt} idx -  Txo (transaction output) SID.
/// @returns {TxoRef}
pub unsafe extern "system" fn Java_com_findora_JniApi_txoRefAbsolute(
    _env: JNIEnv,
    _: JClass,
    idx: jint,
) -> jlong {
    Box::into_raw(Box::new(TxoRef::absolute(idx as u64))) as jlong
}

#[no_mangle]
/// Builds a client record from a JSON-encoded JavaScript value.
///
/// @param {JsValue} val - JSON-encoded autehtnicated asset record fetched from ledger server with the `utxo_sid/{sid}` route,
/// where `sid` can be fetched from the query server with the `get_owned_utxos/{address}` route.
/// Note: The first field of an asset record is `utxo`. See the example below.
///
/// @example
/// "utxo":{
///   "amount":{
///     "NonConfidential":5
///   },
///  "asset_type":{
///     "NonConfidential":[113,168,158,149,55,64,18,189,88,156,133,204,156,46,106,46,232,62,69,233,157,112,240,132,164,120,4,110,14,247,109,127]
///   },
///   "public_key":"Glf8dKF6jAPYHzR_PYYYfzaWqpYcMvnrIcazxsilmlA="
/// }
///
/// @see {@link module:Findora-Network~Network#getUtxo|Network.getUtxo} for information about how to
/// fetch an asset record from the ledger server.
pub unsafe extern "system" fn Java_com_findora_JniApi_clientAssetRecordFromJson(
    env: JNIEnv,
    _: JClass,
    val: JString,
) -> jlong {
    let val: String = env
        .get_string(val)
        .expect("Couldn't get java string!")
        .into();

    Box::into_raw(Box::new(
        ClientAssetRecord::from_json(val.as_str()).unwrap(),
    )) as jlong
}

#[no_mangle]
/// ClientAssetRecord ==> JsValue
pub unsafe extern "system" fn Java_com_findora_JniApi_clientAssetRecordToJson(
    env: JNIEnv,
    _: JClass,
    record: jlong,
) -> jstring {
    let record = &*(record as *mut ClientAssetRecord);
    let output = env
        .new_string(record.to_json().unwrap())
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Builds an owner memo from a JSON-serialized JavaScript value.
/// @param {JsValue} val - JSON owner memo fetched from query server with the `get_owner_memo/{sid}` route,
/// where `sid` can be fetched from the query server with the `get_owned_utxos/{address}` route. See the example below.
///
/// @example
/// {
///   "blind_share":[91,251,44,28,7,221,67,155,175,213,25,183,70,90,119,232,212,238,226,142,159,200,54,19,60,115,38,221,248,202,74,248],
///   "lock":{"ciphertext":[119,54,117,136,125,133,112,193],"encoded_rand":"8KDql2JphPB5WLd7-aYE1bxTQAcweFSmrqymLvPDntM="}
/// }
pub unsafe extern "system" fn Java_com_findora_JniApi_ownerMemoFromJson(
    env: JNIEnv,
    _: JClass,
    val: JString,
) -> jlong {
    let val: String = env
        .get_string(val)
        .expect("Couldn't get java string!")
        .into();

    let zei_owner_memo: ZeiOwnerMemo = serde_json::from_str(val.as_str()).unwrap();
    Box::into_raw(Box::new(OwnerMemo::from_json(zei_owner_memo).unwrap())) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_findora_JniApi_assetTracerKeyPairNew(
    _env: JNIEnv,
    _: JClass,
) -> jlong {
    Box::into_raw(Box::new(AssetTracerKeyPair::new())) as jlong
}

#[no_mangle]
/// Create a new transfer operation builder.
/// @returns {TransferOperationBuilder}
pub extern "system" fn Java_com_findora_JniApi_transferOperationBuilderNew(
    _env: JNIEnv,
    _: JClass,
) -> jlong {
    Box::into_raw(Box::new(TransferOperationBuilder::new())) as jlong
}

#[no_mangle]
/// Debug function that does not need to go into the docs.
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderDebug(
    env: JNIEnv,
    _: JClass,
    builder: jlong,
) -> jstring {
    let builder = &mut *(builder as *mut TransferOperationBuilder);
    let output = env
        .new_string(builder.debug())
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Wraps around TransferOperationBuilder to add an input to a transfer operation builder.
//  @param {TxoRef} txo_ref - Absolute or relative utxo reference
//  @param {string} asset_record - Serialized client asset record to serve as transfer input. This record must exist on the
//  ledger for the transfer to be valid.
//  @param {OwnerMemo} owner_memo - Opening parameters.
//  @param tracing_key {AssetTracerKeyPair} - Tracing key, must be added to traceable
//  assets.
//  @param {XfrKeyPair} key - Key pair associated with the input.
//  @param {BigInt} amount - Amount of input record to transfer.
//  @see {@link module:Findora-Wasm~TxoRef#create_absolute_txo_ref|TxoRef.create_absolute_txo_ref}
//  or {@link module:Findora-Wasm~TxoRef#create_relative_txo_ref|TxoRef.create_relative_txo_ref} for details on txo
//  references.
//  @see {@link module:Findora-Network~Network#getUtxo|Network.getUtxo} for details on fetching blind asset records.
//  @throws Will throw an error if `oar` or `txo_ref` fail to deserialize.
//  @param {TxoRef} txo_ref
//  @param {ClientAssetRecord} asset_record
//  @param {OwnerMemo | undefined} owner_memo
//  @param {TracingPolicies} tracing_policies
//  @param {XfrKeyPair} key
//  @param {BigInt} amount
//  @returns {TransferOperationBuilder}
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderAddInputWithTracing(
    _env: JNIEnv,
    _: JClass,
    builder: jlong,
    txo_ref_ptr: jlong,
    asset_record_ptr: jlong,
    owner_memo_ptr: jlong,
    tracing_policies_ptr: jlong,
    key_ptr: jlong,
    amount: jint,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let txo_ref = *(txo_ref_ptr as *mut TxoRef);
    let asset_record = &*(asset_record_ptr as *mut ClientAssetRecord);
    let owner_memo;
    if 0 == owner_memo_ptr {
        owner_memo = None;
    } else {
        let memo = &*(owner_memo_ptr as *mut OwnerMemo);
        owner_memo = Some(memo.clone());
    }
    let tracing_policies = &*(tracing_policies_ptr as *mut TracingPolicies);
    let key = &*(key_ptr as *mut XfrKeyPair);

    let builder = builder
        .clone()
        .add_input_with_tracing(
            txo_ref,
            asset_record.clone(),
            owner_memo,
            tracing_policies,
            key,
            amount as u64,
        )
        .unwrap();
    Box::into_raw(Box::new(builder)) as jlong
}

#[no_mangle]
/// Wraps around TransferOperationBuilder to add an input to a transfer operation builder.
// * @param {TxoRef} txo_ref - Absolute or relative utxo reference
// * @param {string} asset_record - Serialized client asset record to serve as transfer input. This record must exist on the
// * ledger for the transfer to be valid
// * @param {OwnerMemo} owner_memo - Opening parameters.
// * @param {XfrKeyPair} key - Key pair associated with the input.
// * @param {BigInt} amount - Amount of input record to transfer
// * or {@link module:Findora-Wasm~TxoRef#create_relative_txo_ref|TxoRef.create_relative_txo_ref} for details on txo
// * references.
// * @see {@link module:Findora-Network~Network#getUtxo|Network.getUtxo} for details on fetching blind asset records.
// * @throws Will throw an error if `oar` or `txo_ref` fail to deserialize.
// * @param {TxoRef} txo_ref
// * @param {ClientAssetRecord} asset_record
// * @param {OwnerMemo | undefined} owner_memo
// * @param {XfrKeyPair} key
// * @param {BigInt} amount
// * @returns {TransferOperationBuilder}
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderAddInputNoTracing(
    _env: JNIEnv,
    _: JClass,
    builder: jlong,
    txo_ref_ptr: jlong,
    asset_record_ptr: jlong,
    owner_memo_ptr: jlong,
    key_ptr: jlong,
    amount: jint,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let txo_ref = *(txo_ref_ptr as *mut TxoRef);
    let asset_record = &*(asset_record_ptr as *mut ClientAssetRecord);
    let owner_memo;
    if 0 == owner_memo_ptr {
        owner_memo = None;
    } else {
        let memo = &*(owner_memo_ptr as *mut OwnerMemo);
        owner_memo = Some(memo.clone());
    }
    let key = &*(key_ptr as *mut XfrKeyPair);

    let builder = builder
        .clone()
        .add_input_no_tracing(txo_ref, asset_record, owner_memo, key, amount as u64)
        .unwrap();
    Box::into_raw(Box::new(builder)) as jlong
}

#[no_mangle]
/// Wraps around TransferOperationBuilder to add an output to a transfer operation builder.
// * @param {BigInt} amount - amount to transfer to the recipient.
// * @param {XfrPublicKey} recipient - public key of the recipient.
// * @param tracing_key {AssetTracerKeyPair} - Optional tracing key, must be added to traced
// * assets.
// * @param code {string} - String representation of the asset token code.
// * @param conf_amount {boolean} - `true` means the output's asset amount is confidential, and `false` means it's nonconfidential.
// * @param conf_type {boolean} - `true` means the output's asset type is confidential, and `false` means it's nonconfidential.
// * @throws Will throw an error if `code` fails to deserialize.
// * @param {BigInt} amount
// * @param {XfrPublicKey} recipient
// * @param {TracingPolicies} tracing_policies
// * @param {string} code
// * @param {boolean} conf_amount
// * @param {boolean} conf_type
// * @returns {TransferOperationBuilder}
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderAddOutputWithTracing(
    env: JNIEnv,
    _: JClass,
    builder: jlong,
    amount: jint,
    recipient: jlong,
    tracing_policies_ptr: jlong,
    code: JString,
    conf_amount: jboolean,
    conf_type: jboolean,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let tracing_policies = &*(tracing_policies_ptr as *mut TracingPolicies);
    let recipient = &*(recipient as *mut XfrPublicKey);
    let code: String = env
        .get_string(code)
        .expect("Couldn't get java string!")
        .into();

    let builder = builder
        .clone()
        .add_output_with_tracing(
            amount as u64,
            recipient,
            tracing_policies,
            code,
            conf_amount == JNI_TRUE,
            conf_type == JNI_TRUE,
        )
        .unwrap();
    Box::into_raw(Box::new(builder)) as jlong
}

#[no_mangle]
/// Wraps around TransferOperationBuilder to add an output to a transfer operation builder.
// * @param {BigInt} amount - amount to transfer to the recipient
// * @param {XfrPublicKey} recipient - public key of the recipient
// * @param code {string} - String representaiton of the asset token code
// * @param conf_amount {boolean} - `true` means the output's asset amount is confidential, and `false` means it's nonconfidential.
// * @param conf_type {boolean} - `true` means the output's asset type is confidential, and `false` means it's nonconfidential.
// * @throws Will throw an error if `code` fails to deserialize.
// * @param {BigInt} amount
// * @param {XfrPublicKey} recipient
// * @param {string} code
// * @param {boolean} conf_amount
// * @param {boolean} conf_type
// * @returns {TransferOperationBuilder}
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderAddOutputNoTracing(
    env: JNIEnv,
    _: JClass,
    builder: jlong,
    amount: jint,
    recipient: jlong,
    code: JString,
    conf_amount: jboolean,
    conf_type: jboolean,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let recipient = &*(recipient as *mut XfrPublicKey);
    let code: String = env
        .get_string(code)
        .expect("Couldn't get java string!")
        .into();

    let builder = builder
        .clone()
        .add_output_no_tracing(
            amount as u64,
            recipient,
            code,
            conf_amount == JNI_TRUE,
            conf_type == JNI_TRUE,
        )
        .unwrap();
    Box::into_raw(Box::new(builder)) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderAddInput(
    _env: JNIEnv,
    _: JClass,
    builder: jlong,
    txo_ref_ptr: jlong,
    asset_record_ptr: jlong,
    owner_memo_ptr: jlong,
    tracing_policies_ptr: jlong,
    key_ptr: jlong,
    amount: jint,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let txo_ref = *(txo_ref_ptr as *mut TxoRef);
    let asset_record = &*(asset_record_ptr as *mut ClientAssetRecord);
    let owner_memo;
    if 0 == owner_memo_ptr {
        owner_memo = None;
    } else {
        let memo = &*(owner_memo_ptr as *mut OwnerMemo);
        owner_memo = Some(memo.clone());
    }
    let tracing_policies;
    if 0 == tracing_policies_ptr {
        tracing_policies = None;
    } else {
        let policies = &*(tracing_policies_ptr as *mut TracingPolicies);
        tracing_policies = Some(policies);
    }
    let key = &*(key_ptr as *mut XfrKeyPair);

    let builder = builder
        .clone()
        .add_input(
            txo_ref,
            asset_record,
            owner_memo,
            tracing_policies,
            key,
            amount as u64,
        )
        .unwrap();
    Box::into_raw(Box::new(builder)) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderAddOutput(
    env: JNIEnv,
    _: JClass,
    builder: jlong,
    amount: jint,
    recipient: jlong,
    tracing_policies_ptr: jlong,
    code: JString,
    conf_amount: jboolean,
    conf_type: jboolean,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let tracing_policies;
    if 0 == tracing_policies_ptr {
        tracing_policies = None;
    } else {
        let policies = &*(tracing_policies_ptr as *mut TracingPolicies);
        tracing_policies = Some(policies);
    }
    let recipient = &*(recipient as *mut XfrPublicKey);
    let code: String = env
        .get_string(code)
        .expect("Couldn't get java string!")
        .into();

    let builder = builder
        .clone()
        .add_output(
            amount as u64,
            recipient,
            tracing_policies,
            code,
            conf_amount == JNI_TRUE,
            conf_type == JNI_TRUE,
        )
        .unwrap();
    Box::into_raw(Box::new(builder)) as jlong
}

#[no_mangle]
/// Wraps around TransferOperationBuilder to ensure the transfer inputs and outputs are balanced.
/// This function will add change outputs for all unspent portions of input records.
/// @throws Will throw an error if the transaction cannot be balanced.
/// @returns {TransferOperationBuilder}
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderBalance(
    _env: JNIEnv,
    _: JClass,
    builder: jlong,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    Box::into_raw(Box::new(builder.clone().balance().unwrap())) as jlong
}

#[no_mangle]
/// Wraps around TransferOperationBuilder to finalize the transaction.
/// @throws Will throw an error if input and output amounts do not add up.
/// @throws Will throw an error if not all record owners have signed the transaction.
/// @returns {TransferOperationBuilder}
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderCreate(
    _env: JNIEnv,
    _: JClass,
    builder: jlong,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    Box::into_raw(Box::new(builder.clone().create().unwrap())) as jlong
}

#[no_mangle]
/// Wraps around TransferOperationBuilder to add a signature to the operation.
///
/// All input owners must sign.
///
/// @param {XfrKeyPair} kp - key pair of one of the input owners.
/// @param {XfrKeyPair} kp
/// @returns {TransferOperationBuilder}
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderSign(
    _env: JNIEnv,
    _: JClass,
    builder: jlong,
    key_ptr: jlong,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let key = &*(key_ptr as *mut XfrKeyPair);

    Box::into_raw(Box::new(builder.clone().sign(key).unwrap())) as jlong
}

#[no_mangle]
/// Co-sign an input index
/// @param {XfrKeyPair} kp - Co-signature key.
/// @params {Number} input_idx - Input index to apply co-signature to.
/// @param {XfrKeyPair} kp
/// @param {number} input_idx
/// @returns {TransferOperationBuilder}
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderAddCosignature(
    _env: JNIEnv,
    _: JClass,
    builder: jlong,
    key_ptr: jlong,
    input_idx: jint,
) -> jlong {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let key = &*(key_ptr as *mut XfrKeyPair);

    Box::into_raw(Box::new(
        builder
            .clone()
            .add_cosignature(key, input_idx as usize)
            .unwrap(),
    )) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderBuilder(
    env: JNIEnv,
    _: JClass,
    builder: jlong,
) -> jstring {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let output = env
        .new_string(builder.builder())
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Wraps around TransferOperationBuilder to extract an operation expression as JSON.
pub unsafe extern "system" fn Java_com_findora_JniApi_transferOperationBuilderTransaction(
    env: JNIEnv,
    _: JClass,
    builder: jlong,
) -> jstring {
    let builder = &*(builder as *mut TransferOperationBuilder);
    let output = env
        .new_string(builder.transaction().unwrap())
        .expect("Couldn't create java string!");
    output.into_inner()
}

#[no_mangle]
/// Fee smaller than this value will be denied.
/// @returns {BigInt}
pub unsafe extern "system" fn Java_com_findora_JniApi_fraGetMinimalFee(
    _env: JNIEnv,
    _: JClass,
) -> jvalue {
    jvalue {
        _data: fra_get_minimal_fee(),
    }
}

#[no_mangle]
/// The destination for fee to be transfered to.
/// @returns {XfrPublicKey}
pub unsafe extern "system" fn Java_com_findora_JniApi_fraGetDestPubkey(
    _env: JNIEnv,
    _: JClass,
) -> jlong {
    Box::into_raw(Box::new(fra_get_dest_pubkey())) as jlong
}
