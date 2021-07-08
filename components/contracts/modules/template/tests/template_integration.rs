//! Template module integration tests.
use abci::*;
use baseapp::{Action, BaseApp, UncheckedTransaction};
use fp_core::crypto::{Address32, MultiSignature};
use lazy_static::lazy_static;
use module_template::ValueStore;
use parking_lot::RwLock;
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};
use std::{
    env::temp_dir,
    sync::{Arc, Mutex},
    time::SystemTime,
};
use storage::{db::FinDB, state::ChainState};
use zei::xfr::sig::XfrKeyPair;

lazy_static! {
    static ref BASE_APP: Mutex<BaseApp> =
        Mutex::new(BaseApp::new(create_temp_db()).unwrap());
}

#[test]
fn run_all_tests() {
    test_abci_info();
    test_abci_init_chain();
    test_abci_check_tx();
    test_abci_begin_block();
    test_abci_deliver_tx();
    test_abci_end_block();
    test_abci_commit();
    test_abci_query();
}

fn build_signed_transaction(function: Action) -> UncheckedTransaction {
    let mut prng = ChaChaRng::from_entropy();
    let alice = XfrKeyPair::generate(&mut prng);
    let signer = Address32::from(alice.get_pk());
    let msg = serde_json::to_vec(&function).unwrap();
    let sig = alice.get_sk_ref().sign(msg.as_slice(), alice.get_pk_ref());
    let signature = MultiSignature::from(sig);

    UncheckedTransaction::new_signed(function, signer, signature)
}

fn create_temp_db() -> Arc<RwLock<ChainState<FinDB>>> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = temp_dir();
    path.push(format!("temp-findora-db–{}", time));
    let fdb = FinDB::open(path).unwrap();
    Arc::new(RwLock::new(ChainState::new(fdb, "temp_db".to_string())))
}

fn test_abci_info() {
    let resp = BASE_APP.lock().unwrap().info(&RequestInfo::default());
    assert_eq!(resp.data, "findora".to_string());
    assert_eq!(resp.version, "1.0.0".to_string());
    assert_eq!(resp.app_version, 1);
    assert_eq!(resp.last_block_height, 0);
}

fn test_abci_init_chain() {
    let mut req = RequestInitChain::new();
    req.set_chain_id("findora test".to_string());
    let _ = BASE_APP.lock().unwrap().init_chain(&req);

    assert_eq!(
        BASE_APP.lock().unwrap().deliver_state.chain_id(),
        req.chain_id
    );
    assert_eq!(BASE_APP.lock().unwrap().deliver_state.block_height(), 0);
}

fn test_abci_check_tx() {
    let mut req = RequestCheckTx::new();

    let function = Action::Template(module_template::Action::SetValue(10));
    req.tx = serde_json::to_vec(&build_signed_transaction(function)).unwrap();
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
    let function = Action::Template(module_template::Action::SetValue(10));
    req.tx = serde_json::to_vec(&build_signed_transaction(function)).unwrap();
    let resp = BASE_APP.lock().unwrap().deliver_tx(&req);
    assert_eq!(
        resp.code, 0,
        "deliver tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    assert_eq!(
        ValueStore::get(BASE_APP.lock().unwrap().deliver_state.store.clone()),
        Some(10)
    );
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

fn test_abci_query() {
    let mut req = RequestQuery::new();
    req.path = String::from("module/template/value");
    let resp = BASE_APP.lock().unwrap().query(&req);

    assert_eq!(
        resp.code, 0,
        "query tx failed, code: {}, log: {}",
        resp.code, resp.log
    );

    assert_eq!(
        serde_json::from_slice::<u64>(resp.value.as_slice()).unwrap(),
        10
    );
}
