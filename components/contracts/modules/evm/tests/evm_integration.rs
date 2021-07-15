mod utils;

use abci::*;
use baseapp::{Action, BaseApp, ChainId, UncheckedTransaction};
use fp_core::crypto::Address;
use fp_traits::account::AccountAsset;
use fp_utils::db::create_temp_db_path;
use fp_utils::ethereum::*;
use lazy_static::lazy_static;
use ledger::data_model::ASSET_TYPE_FRA;
use primitive_types::{H160, U256};
use std::sync::Mutex;
use utils::erc20::*;

lazy_static! {
    static ref BASE_APP: Mutex<BaseApp> =
        Mutex::new(BaseApp::new(create_temp_db_path().as_path()).unwrap());
    static ref ALICE: KeyPair = generate_address(1);
    static ref BOB: KeyPair = generate_address(2);
}

fn build_erc20_deploy_transaction(
    name: &str,
    symbol: &str,
    nonce: U256,
) -> UncheckedTransaction {
    let constructor = ERC20Constructor::load();
    let tx = constructor.deploy(name, symbol, nonce);
    let raw_tx = tx.sign(&ALICE.private_key, ChainId::get());
    let function = Action::Ethereum(module_ethereum::Action::Transact(raw_tx));
    UncheckedTransaction::new_unsigned(function)
}

#[test]
fn initialize_erc20_works() {
    test_mint_balance(&ALICE.account_id, 2000000, 1);
    test_abci_check_tx();
    test_abci_deliver_tx();
    // test_abci_end_block();
    // test_abci_commit();
    // test_abci_query()
}

fn test_abci_check_tx() {
    let mut req = RequestCheckTx::new();
    req.tx =
        serde_json::to_vec(&build_erc20_deploy_transaction("erc20", "FRA", 0.into()))
            .unwrap();
    let resp = BASE_APP.lock().unwrap().check_tx(&req);
    assert_eq!(
        resp.code, 0,
        "check tx failed, code: {}, log: {}",
        resp.code, resp.log
    );
}

fn test_abci_deliver_tx() {
    let mut req = RequestDeliverTx::new();
    req.tx =
        serde_json::to_vec(&build_erc20_deploy_transaction("erc20", "FRA", 0.into()))
            .unwrap();
    let resp = BASE_APP.lock().unwrap().deliver_tx(&req);
    assert_eq!(
        resp.code, 0,
        "deliver tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    // // initial balance = 2000000, gas fee = 21000, transfer balance = 10
    // assert_eq!(
    //     module_account::App::<BaseApp>::balance(
    //         &BASE_APP.lock().unwrap().deliver_state,
    //         &ALICE.account_id
    //     ),
    //     2000000 - 21000 - 10
    // );
    //
    // assert_eq!(
    //     module_account::App::<BaseApp>::balance(
    //         &BASE_APP.lock().unwrap().deliver_state,
    //         &BOB.account_id
    //     ),
    //     10
    // );
}

fn test_mint_balance(who: &Address, balance: u128, height: u64) {
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
