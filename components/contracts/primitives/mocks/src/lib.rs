#![cfg_attr(test, allow(unused_imports))]

pub use baseapp::{Action, BaseApp, CheckFee, CheckNonce, UncheckedTransaction};

use fp_core::crypto::{Address, MultiSignature};
use fp_traits::account::AccountAsset;
use fp_utils::db::create_temp_db_path;
use fp_utils::ethereum::{generate_address, KeyPair};
use lazy_static::lazy_static;
use ledger::data_model::ASSET_TYPE_FRA;
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};
use std::sync::Mutex;
use zei::xfr::sig::XfrKeyPair;

lazy_static! {
    pub static ref BASE_APP: Mutex<BaseApp> =
        Mutex::new(BaseApp::new(create_temp_db_path().as_path()).unwrap());
    pub static ref ALICE_ECDSA: KeyPair = generate_address(1);
    pub static ref BOB_ECDSA: KeyPair = generate_address(2);
    pub static ref ALICE_XFR: XfrKeyPair =
        XfrKeyPair::generate(&mut ChaChaRng::from_entropy());
    pub static ref BOB_XFR: XfrKeyPair =
        XfrKeyPair::generate(&mut ChaChaRng::from_entropy());
}

pub fn test_mint_balance(who: &Address, balance: u128, height: u64) {
    assert!(
        module_account::App::<BaseApp>::mint(
            &BASE_APP.lock().unwrap().deliver_state,
            who,
            balance,
            ASSET_TYPE_FRA
        )
        .is_ok()
    );
    BASE_APP
        .lock()
        .unwrap()
        .deliver_state
        .store
        .clone()
        .write()
        .commit(height)
        .unwrap();

    let ctx = BASE_APP
        .lock()
        .unwrap()
        .create_query_context(height, false)
        .unwrap();
    assert_eq!(module_account::App::<BaseApp>::balance(&ctx, who), balance);
}

pub fn build_signed_transaction(
    function: Action,
    who: &XfrKeyPair,
    nonce: u64,
) -> UncheckedTransaction {
    let extra = (CheckNonce::new(nonce), CheckFee::new(None));

    let signer: Address = who.get_pk().into();
    let msg = serde_json::to_vec(&(function.clone(), extra.clone())).unwrap();
    let sig = who.get_sk_ref().sign(msg.as_slice(), who.get_pk_ref());
    let signature = MultiSignature::from(sig);

    UncheckedTransaction::new_signed(function, signer, signature, extra)
}
