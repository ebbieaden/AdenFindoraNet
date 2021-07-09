//! Ethereum module integration tests.

mod mock;

use abci::*;
use baseapp::{Action, BaseApp, UncheckedTransaction};
use lazy_static::lazy_static;
use mock::*;
use primitive_types::{H160, U256};
use std::sync::Mutex;

lazy_static! {
    static ref BASE_APP: Mutex<BaseApp> =
        Mutex::new(BaseApp::new(create_temp_db()).unwrap());
}

#[test]
fn run_all_tests() {
    test_abci_check_tx();
    test_abci_begin_block();
    test_abci_deliver_tx();
    test_abci_end_block();
    test_abci_commit();
    // test_abci_query();
}

fn build_transfer_transaction(to: H160, balance: u128) -> UncheckedTransaction {
    let tx = UnsignedTransaction {
        nonce: U256::zero(),
        gas_price: U256::from(1),
        gas_limit: U256::from(0x100000),
        action: ethereum::TransactionAction::Call(to),
        value: U256::from(balance),
        input: Vec::new(),
    };

    let account = address_build(1);
    let raw_tx = tx.sign(&account.private_key, CHAIN_ID);
    let function = Action::Ethereum(module_ethereum::Action::Transact(raw_tx));
    UncheckedTransaction::new_unsigned(function)
}

fn test_abci_check_tx() {
    let mut req = RequestCheckTx::new();
    let account = address_build(2);
    req.tx =
        serde_json::to_vec(&build_transfer_transaction(account.address, 10)).unwrap();
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
    header.height = 1;
    req.set_header(header.clone());
    let _ = BASE_APP.lock().unwrap().begin_block(&req);
    let _ = BASE_APP.lock().unwrap().commit(&RequestCommit::new());
    header.height = 2;
    req.set_header(header.clone());
    let _ = BASE_APP.lock().unwrap().begin_block(&req);
}

fn test_abci_deliver_tx() {
    let mut req = RequestDeliverTx::new();
    let account = address_build(2);
    req.tx =
        serde_json::to_vec(&build_transfer_transaction(account.address, 10)).unwrap();
    let resp = BASE_APP.lock().unwrap().deliver_tx(&req);
    assert_eq!(
        resp.code, 0,
        "deliver tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    // assert_eq!(
    //     ValueStore::get(BASE_APP.lock().unwrap().deliver_state.store.clone()),
    //     Some(10)
    // );
}

fn test_abci_end_block() {
    let mut req = RequestEndBlock::new();
    req.set_height(2);
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
        2
    );
}
