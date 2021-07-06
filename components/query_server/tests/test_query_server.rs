//! test query server

#![allow(warnings)]

use lazy_static::lazy_static;
use ledger::data_model::{
    AssetRules, AssetTypeCode, Operation, Transaction, TxnEffect, TxnSID, TxoSID, Utxo,
    XfrAddress,
};
use ledger::staking::ops::mint_fra::{MintEntry, MintFraOps, MintKind};
use ledger::store::{LedgerAccess, LedgerState, LedgerUpdate};
use metrics_exporter_prometheus::PrometheusHandle;
use parking_lot::RwLock;
use query_server::QueryServer;
use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;
use ruc::*;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use txn_builder::{BuildsTransactions, PolicyChoice, TransactionBuilder};
use utils::MetricsRenderer;
use zei::setup::PublicParams;
use zei::xfr::asset_record::{
    build_blind_asset_record, open_blind_asset_record, AssetRecordType,
};
use zei::xfr::sig::XfrKeyPair;
use zei::xfr::structs::OwnerMemo;

lazy_static! {
    static ref LEDGER: Arc<RwLock<LedgerState>> =
        Arc::new(RwLock::new(LedgerState::test_ledger()));
    static ref QS: Arc<RwLock<QueryServer<PromHandle>>> =
        Arc::new(RwLock::new(create_server()));
}

struct PromHandle(metrics_exporter_prometheus::PrometheusHandle);

impl PromHandle {
    pub fn new(h: PrometheusHandle) -> PromHandle {
        PromHandle(h)
    }
}

impl MetricsRenderer for PromHandle {
    fn rendered(&self) -> String {
        self.0.render()
    }
}

fn create_server() -> QueryServer<PromHandle> {
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    let recorder = builder.build();
    let handle = PromHandle::new(recorder.handle());

    let mut qs = QueryServer::new(LEDGER.clone(), handle);
    qs
}

fn apply_transaction(tx: Transaction) -> Option<(TxnSID, Vec<TxoSID>)> {
    let effect = pnk!(TxnEffect::compute_effect(tx));
    let mut ledger = LEDGER.write();
    let mut block = pnk!(ledger.start_block());
    let temp_sid = pnk!(ledger.apply_transaction(&mut block, effect, false));
    pnk!(ledger.finish_block(block)).remove(&temp_sid)
}

/// process
/// *. define
/// *. issue
/// *. mint
/// test query sever function
/// 1. get_address_of_sid
/// 2. get_owned_utxo_sids
/// 3. get_coinbase_entries
fn test_scene_1() -> Result<()> {
    let mut prng = ChaChaRng::from_entropy();
    let code = AssetTypeCode::gen_random();
    let x_kp = XfrKeyPair::generate(&mut prng);
    let params = PublicParams::default();

    // define
    let mut builder =
        TransactionBuilder::from_seq_id(LEDGER.read().get_block_commit_count());
    let tx = pnk!(builder.add_operation_create_asset(
        &x_kp,
        Some(code),
        AssetRules::default(),
        "test",
        PolicyChoice::Fungible(),
    ))
    .transaction();
    pnk!(apply_transaction(tx.clone()));

    // issue
    let mut builder =
        TransactionBuilder::from_seq_id(LEDGER.read().get_block_commit_count());
    let tx = pnk!(builder.add_basic_issue_asset(
        &x_kp,
        &code,
        LEDGER.read().get_block_commit_count(),
        1000,
        AssetRecordType::NonConfidentialAmount_NonConfidentialAssetType,
        &params,
    ))
    .transaction();
    let (_, issue_txos) = pnk!(apply_transaction(tx.clone()));

    // create txn from mint_ops
    let mint_ops = Operation::MintFra(MintFraOps::new(
        0u64,
        vec![
            MintEntry::new(MintKind::Claim, x_kp.pub_key, None, 100),
            MintEntry::new(MintKind::UnStake, x_kp.pub_key, None, 900),
        ],
    ));
    let tx =
        Transaction::from_operation(mint_ops, LEDGER.read().get_block_commit_count());
    let (_, mint_txos) = pnk!(apply_transaction(tx.clone()));

    // A necessary step, in fact, is to update utxos_to_map_index
    QS.write().update();

    let result = QS
        .read()
        .get_coinbase_entries(&XfrAddress { key: x_kp.pub_key }, 0, 5, true)
        .unwrap();

    // judgement api resp
    let judgement_mint_result =
        move |result: Vec<(u64, MintEntry)>, mut amounts: Vec<u64>| {
            amounts.reverse();
            for (amount, (_block_height, mint_entry)) in
                amounts.iter().zip(result.iter())
            {
                assert_eq!(*amount, mint_entry.amount);
                assert_eq!(Some(*amount), mint_entry.utxo.record.amount.get_amount());
            }
        };
    judgement_mint_result(result, vec![100, 900]);

    // test call api
    let op = QS.read().get_address_of_sid(issue_txos[0]).cloned();
    assert_eq!(Some(XfrAddress { key: x_kp.pub_key }), op);

    for mint_txo in mint_txos {
        let op = QS.read().get_address_of_sid(mint_txo).cloned();
        assert_eq!(Some(XfrAddress { key: x_kp.pub_key }), op);
    }

    let op = QS
        .read()
        .get_owned_utxo_sids(&XfrAddress {
            key: x_kp.pub_key.clone(),
        })
        .cloned();

    let map = LEDGER.read().get_owned_utxos(&x_kp.get_pk());
    let judgement_get_utxo_sids_result =
        move |set: HashSet<TxoSID>, map: BTreeMap<TxoSID, (Utxo, Option<OwnerMemo>)>| {
            for txo_sid in set.iter() {
                assert!(map.get(txo_sid).is_some())
            }
        };
    judgement_get_utxo_sids_result(op.unwrap(), map);
    Ok(())
}

#[test]
fn test() {
    pnk!(test_scene_1());
}
