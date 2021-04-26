use crate::rust::types;
use crate::rust::*;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jbyteArray, jlong, jstring};
use jni::JNIEnv;
use zei::xfr::structs::ASSET_TYPE_LENGTH;

#[no_mangle]
pub extern "system" fn Java_com_findora_JniApi_buildId(
    env: JNIEnv,
    _: JClass,
) -> jstring {
    let output = env
        .new_string(build_id())
        .expect("Couldn't create java string!");
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

// TODO
#[no_mangle]
pub unsafe extern "system" fn Java_com_findora_JniApi_authenticatedKVLookupNew(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    unimplemented!()
    // let val = types::AuthenticatedKVLookup{
    // };
    //
    // Box::into_raw(Box::new(val)) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_findora_JniApi_authenticatedKVLookupDestroy(
    _env: JNIEnv,
    _class: JClass,
    authenticated_res_ptr: jlong,
) {
    let _boxed_authenticated_res =
        Box::from_raw(authenticated_res_ptr as *mut types::AuthenticatedKVLookup);
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

    rs_verify_authenticated_custom_data_result(state_commitment, &res)
        .unwrap_or(false) as jboolean
}
