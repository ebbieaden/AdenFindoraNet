//! Ethereum module integration tests.

use abci::*;
use baseapp::{Action, BaseApp, ChainId, UncheckedTransaction};
use fp_core::crypto::Address;
use fp_traits::{
    account::AccountAsset,
    evm::{DecimalsMapping, FeeCalculator},
};
use fp_utils::{db::create_temp_db_path, ethereum::*};
use lazy_static::lazy_static;
use ledger::data_model::ASSET_TYPE_FRA;
use primitive_types::{H160, U256};
use std::sync::Mutex;

lazy_static! {
    static ref BASE_APP: Mutex<BaseApp> =
        Mutex::new(BaseApp::new(create_temp_db_path().as_path()).unwrap());
    static ref ALICE: KeyPair = generate_address(1);
    static ref BOB: KeyPair = generate_address(2);
}

#[test]
fn run_all_tests() {
    test_abci_check_tx();
    test_abci_begin_block();
    test_abci_deliver_tx();
    test_abci_end_block();
    test_abci_commit();
    test_abci_query()
}

fn build_transfer_transaction(to: H160, balance: u128) -> UncheckedTransaction {
    let tx = UnsignedTransaction {
        nonce: U256::zero(),
        gas_price: <BaseApp as module_evm::Config>::FeeCalculator::min_gas_price(),
        gas_limit: U256::from(0x100000),
        action: ethereum::TransactionAction::Call(to),
        value: U256::from(balance),
        input: Vec::new(),
    };

    let raw_tx = tx.sign(&ALICE.private_key, ChainId::get());
    let function = Action::Ethereum(module_ethereum::Action::Transact(raw_tx));
    UncheckedTransaction::new_unsigned(function)
}

fn test_abci_check_tx() {
    let mut req = RequestCheckTx::new();
    let value =
        <BaseApp as module_evm::Config>::DecimalsMapping::from_native_token(10.into())
            .unwrap();
    req.tx =
        serde_json::to_vec(&build_transfer_transaction(BOB.address, value.as_u128()))
            .unwrap();
    let resp = BASE_APP.lock().unwrap().check_tx(&req);
    assert!(
        resp.code == 1 && resp.log.contains("InvalidTransaction: InsufficientBalance"),
        "resp log: {}",
        resp.log
    );

    test_mint_balance(&ALICE.account_id, 2000000, 2);

    let resp = BASE_APP.lock().unwrap().check_tx(&req);
    assert_eq!(
        resp.code, 0,
        "check tx failed, code: {}, log: {}",
        resp.code, resp.log
    );
}

fn test_abci_begin_block() {
    let mut req = RequestBeginBlock::new();
    req.hash = b"test".to_vec();
    let mut header = Header::new();
    header.height = 3;
    req.set_header(header.clone());
    let _ = BASE_APP.lock().unwrap().begin_block(&req);
}

fn test_abci_deliver_tx() {
    let mut req = RequestDeliverTx::new();
    let value =
        <BaseApp as module_evm::Config>::DecimalsMapping::from_native_token(10.into())
            .unwrap();
    req.tx =
        serde_json::to_vec(&build_transfer_transaction(BOB.address, value.as_u128()))
            .unwrap();
    let resp = BASE_APP.lock().unwrap().deliver_tx(&req);
    assert_eq!(
        resp.code, 0,
        "deliver tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    println!("transfer resp: {:?}", resp);

    // initial balance = 2000000, gas fee = 21000, transfer balance = 10
    assert_eq!(
        module_account::App::<BaseApp>::balance(
            &BASE_APP.lock().unwrap().deliver_state,
            &ALICE.account_id
        ),
        2000000 - 21000 - 10
    );

    assert_eq!(
        module_account::App::<BaseApp>::balance(
            &BASE_APP.lock().unwrap().deliver_state,
            &BOB.account_id
        ),
        10
    );
}

fn test_abci_end_block() {
    let mut req = RequestEndBlock::new();
    req.set_height(3);
    let _ = BASE_APP.lock().unwrap().end_block(&req);
}

fn test_abci_commit() {
    let resp = BASE_APP.lock().unwrap().commit(&RequestCommit::new());
    println!("root hash: {}", hex::encode(resp.data));
    assert_eq!(
        BASE_APP
            .lock()
            .unwrap()
            .chain_state
            .read()
            .height()
            .unwrap(),
        3
    );
}

fn test_abci_query() {
    let ctx = BASE_APP
        .lock()
        .unwrap()
        .create_query_context(3, false)
        .unwrap();
    assert_eq!(
        module_account::App::<BaseApp>::balance(&ctx, &ALICE.account_id),
        2000000 - 21000 - 10
    );

    assert_eq!(
        module_account::App::<BaseApp>::balance(&ctx, &BOB.account_id),
        10
    );
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
