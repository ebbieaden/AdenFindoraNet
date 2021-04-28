mod constructor;

use crate::rust::types;
use crate::rust::*;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jbyteArray, jint, jlong, jstring};
use jni::JNIEnv;
use zei::xfr::structs::ASSET_TYPE_LENGTH;

#[no_mangle]
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
pub extern "system" fn Java_com_findora_JniApi_restore_keypairFromMnemonicDefault(
    env: JNIEnv,
    _: JClass,
    phrase: JString,
) -> jlong {
    let phrase: String = env
        .get_string(phrase)
        .expect("Couldn't get java string!")
        .into();
    let keypair = rs_restore_keypair_from_mnemonic_default(phrase.as_str()).unwrap();
    Box::into_raw(Box::new(types::XfrKeyPair::from(keypair))) as jlong
}

#[no_mangle]
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
    let keypair = create_keypair_from_secret(sk).unwrap();
    Box::into_raw(Box::new(types::XfrKeyPair::from(keypair))) as jlong
}
