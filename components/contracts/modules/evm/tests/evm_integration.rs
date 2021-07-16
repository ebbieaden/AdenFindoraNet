mod utils;

use abci::*;
use baseapp::{Action, BaseApp, ChainId, UncheckedTransaction};
use ethereum_types::H256;
use fp_core::crypto::Address;
use fp_evm::CallOrCreateInfo;
use fp_traits::account::AccountAsset;
use fp_utils::db::create_temp_db_path;
use fp_utils::ethereum::*;
use lazy_static::lazy_static;
use ledger::data_model::ASSET_TYPE_FRA;
use module_evm::Runner;
use module_evm::{storage::*, Call};
use primitive_types::{H160, U256};
use std::sync::Mutex;
use utils::*;

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
) -> (UncheckedTransaction, ethabi::Contract) {
    let constructor = ERC20Constructor::load();
    let tx = constructor.deploy(name, symbol, nonce);
    let raw_tx = tx.sign(&ALICE.private_key, ChainId::get());
    let function = Action::Ethereum(module_ethereum::Action::Transact(raw_tx));
    (
        UncheckedTransaction::new_unsigned(function),
        constructor.0.abi,
    )
}

fn build_erc20_mint_transaction(
    contract: ERC20,
    recipient: H160,
    amount: U256,
    nonce: U256,
) -> UncheckedTransaction {
    let tx = contract.mint(recipient, amount, nonce);
    let raw_tx = tx.sign(&ALICE.private_key, ChainId::get());
    let function = Action::Ethereum(module_ethereum::Action::Transact(raw_tx));
    UncheckedTransaction::new_unsigned(function)
}

fn build_erc20_transfer_transaction(
    contract: ERC20,
    recipient: H160,
    amount: U256,
    nonce: U256,
    value: U256,
    signer: H256,
) -> UncheckedTransaction {
    let tx = contract.transfer(recipient, amount, nonce, value);
    let raw_tx = tx.sign(&signer, ChainId::get());
    let function = Action::Ethereum(module_ethereum::Action::Transact(raw_tx));
    UncheckedTransaction::new_unsigned(function)
}

fn build_erc20_balance_of_transaction(
    contract: ERC20,
    address: H160,
    nonce: U256,
) -> UncheckedTransaction {
    let tx = contract.balance_of(address, nonce);
    let raw_tx = tx.sign(&ALICE.private_key, ChainId::get());
    let function = Action::Ethereum(module_ethereum::Action::Transact(raw_tx));
    UncheckedTransaction::new_unsigned(function)
}

#[test]
fn erc20_works() {
    test_mint_balance(&ALICE.account_id, 10000000, 1);
    test_mint_balance(&BOB.account_id, 10000000, 1);

    // erc20 initialize
    test_deploy_check_tx();
    let (address, abi) = test_deploy_deliver_tx();
    test_deploy_commit(address);

    let erc20_instance = ERC20(DeployedContract { abi, address });
    // erc20 mint
    test_mint_check_tx(erc20_instance.clone());
    test_mint_deliver_tx(erc20_instance.clone());
    assert_eq!(
        test_balance_of_deliver_tx(erc20_instance.clone(), BOB.address),
        10000.into()
    );

    // erc20 transfer
    test_transfer_check_tx(erc20_instance.clone());
    test_transfer_deliver_tx(erc20_instance.clone());
    assert_eq!(
        test_balance_of_with_eth_call(erc20_instance.clone(), BOB.address),
        9900.into()
    );
    assert_eq!(
        test_balance_of_with_eth_call(erc20_instance.clone(), ALICE.address),
        100.into()
    );
}

fn test_deploy_check_tx() {
    let mut req = RequestCheckTx::new();
    req.tx =
        serde_json::to_vec(&build_erc20_deploy_transaction("erc20", "FRA", 0.into()).0)
            .unwrap();
    let resp = BASE_APP.lock().unwrap().check_tx(&req);
    assert_eq!(
        resp.code, 0,
        "erc20 deploy check tx failed, code: {}, log: {}",
        resp.code, resp.log
    );
}

fn test_deploy_deliver_tx() -> (H160, ethabi::Contract) {
    let mut req = RequestDeliverTx::new();
    let (tx, contract_abi) = build_erc20_deploy_transaction("erc20", "FRA", 0.into());

    req.tx = serde_json::to_vec(&tx).unwrap();
    let resp = BASE_APP.lock().unwrap().deliver_tx(&req);
    assert_eq!(
        resp.code, 0,
        "erc20 deploy deliver tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    println!("deploy erc20 result: {:?}\n", resp);

    let info = serde_json::from_slice::<CallOrCreateInfo>(resp.get_data()).unwrap();
    if let CallOrCreateInfo::Create(info) = info {
        assert!(
            info.exit_reason.is_succeed(),
            "erc20 deploy failed: {:?}",
            info.exit_reason
        );

        println!("erc20 contract address: {:?}\n", info.value);
        (info.value, contract_abi)
    } else {
        panic!("not expected result: {:?}", info)
    }
}

fn test_deploy_commit(contract_address: H160) {
    let _ = BASE_APP.lock().unwrap().commit(&RequestCommit::new());

    let ctx = BASE_APP
        .lock()
        .unwrap()
        .create_query_context(0, false)
        .unwrap();
    assert!(AccountCodes::contains_key(ctx.store, &contract_address));
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

fn test_mint_check_tx(contract: ERC20) {
    let mut req = RequestCheckTx::new();
    req.tx = serde_json::to_vec(&build_erc20_mint_transaction(
        contract,
        BOB.address,
        10000.into(),
        1.into(),
    ))
    .unwrap();
    let resp = BASE_APP.lock().unwrap().check_tx(&req);
    assert_eq!(
        resp.code, 0,
        "erc20 mint check tx failed, code: {}, log: {}",
        resp.code, resp.log
    );
}

fn test_mint_deliver_tx(contract: ERC20) {
    let mut req = RequestDeliverTx::new();
    req.tx = serde_json::to_vec(&build_erc20_mint_transaction(
        contract.clone(),
        BOB.address,
        10000.into(),
        1.into(),
    ))
    .unwrap();
    let resp = BASE_APP.lock().unwrap().deliver_tx(&req);
    assert_eq!(
        resp.code, 0,
        "erc20 mint deliver tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    println!("call erc20 mint result: {:?}\n", resp);
}

fn test_transfer_check_tx(contract: ERC20) {
    let mut req = RequestCheckTx::new();
    req.tx = serde_json::to_vec(&build_erc20_transfer_transaction(
        contract,
        ALICE.address,
        100.into(),
        0.into(),
        U256::zero(),
        BOB.private_key,
    ))
    .unwrap();
    let resp = BASE_APP.lock().unwrap().check_tx(&req);
    assert_eq!(
        resp.code, 0,
        "erc20 transfer check tx failed, code: {}, log: {}",
        resp.code, resp.log
    );
}

fn test_transfer_deliver_tx(contract: ERC20) {
    let mut req = RequestDeliverTx::new();
    req.tx = serde_json::to_vec(&build_erc20_transfer_transaction(
        contract,
        ALICE.address,
        100.into(),
        0.into(),
        U256::zero(),
        BOB.private_key,
    ))
    .unwrap();
    let resp = BASE_APP.lock().unwrap().deliver_tx(&req);
    assert_eq!(
        resp.code, 0,
        "erc20 transfer deliver tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    println!("call erc20 transfer result: {:?}\n", resp);
}

fn test_balance_of_deliver_tx(contract: ERC20, who: H160) -> U256 {
    let mut req = RequestDeliverTx::new();
    req.tx =
        serde_json::to_vec(&build_erc20_balance_of_transaction(contract, who, 2.into()))
            .unwrap();
    let resp = BASE_APP.lock().unwrap().deliver_tx(&req);
    assert_eq!(
        resp.code, 0,
        "check tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    println!("call erc20 balanceOf result: {:?}\n", resp);

    let info = serde_json::from_slice::<CallOrCreateInfo>(resp.get_data()).unwrap();
    if let CallOrCreateInfo::Call(info) = info {
        assert!(
            info.exit_reason.is_succeed(),
            "query erc20 balance failed: {:?}",
            info.exit_reason
        );

        U256::from_big_endian(info.value.as_ref())
    } else {
        panic!("not expected result: {:?}", info)
    }
}

fn test_balance_of_with_eth_call(contract: ERC20, who: H160) -> U256 {
    let input = contract
        .0
        .abi
        .function("balanceOf")
        .unwrap()
        .encode_input(&[ethabi::Token::Address(who)])
        .unwrap();

    let call = Call {
        source: ALICE.address,
        target: contract.0.address,
        input,
        value: U256::zero(),
        gas_limit: <BaseApp as module_evm::Config>::BlockGasLimit::get().as_u64(),
        gas_price: None,
        nonce: None,
    };

    let info = <BaseApp as module_evm::Config>::Runner::call(
        &BASE_APP.lock().unwrap().deliver_state.clone(),
        call,
        <BaseApp as module_evm::Config>::config(),
    )
    .unwrap();

    U256::from_big_endian(info.value.as_ref())
}
