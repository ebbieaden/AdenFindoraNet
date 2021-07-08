//! test query server

#![allow(warnings)]

use lazy_static::lazy_static;
use ledger::data_model::{
    Operation, Transaction, TxnEffect, TxnSID, TxoSID, XfrAddress,
};
use ledger::staking::ops::mint_fra::{MintEntry, MintFraOps, MintKind};
use ledger::store::{LedgerAccess, LedgerState, LedgerUpdate};
use metrics_exporter_prometheus::PrometheusHandle;
use parking_lot::RwLock;
use query_server::QueryServer;
use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;
use ruc::*;
use std::sync::Arc;
use utils::MetricsRenderer;
use zei::xfr::sig::XfrKeyPair;

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

/// test query_coinbase_hist
fn test_query_coinbase_hist() -> Result<()> {
    let mut prng = ChaChaRng::from_entropy();
    // create key pair
    let x_kp = XfrKeyPair::generate(&mut prng);
    //mint fra
    let mint_ops = Operation::MintFra(MintFraOps::new(
        0u64,
        vec![
            MintEntry::new(MintKind::Claim, x_kp.pub_key, None, 100),
            MintEntry::new(MintKind::UnStake, x_kp.pub_key, None, 900),
        ],
    ));
    // create txn from mint_ops
    let seq_id = LEDGER.read().get_block_commit_count();
    let tx = Transaction::from_operation(mint_ops, seq_id);
    pnk!(apply_transaction(tx.clone()));

    // A necessary step, in fact, is to update coinbase_oper_hist
    QS.write().update();

    // test call api
    // let result = qs.get_coinbase_entries(&addr, 0, 5, true).unwrap();
    let result = QS
        .read()
        .get_coinbase_entries(&XfrAddress { key: x_kp.pub_key }, 0, 5, true)
        .unwrap();

    // judgement api resp
    let judgement_result = move |result: Vec<(u64, MintEntry)>,
                                 mut amounts: Vec<u64>|
          -> Result<()> {
        amounts.reverse();
        for (amount, (_block_height, mint_entry)) in amounts.iter().zip(result.iter()) {
            assert_eq!(*amount, mint_entry.amount);
            assert_eq!(Some(*amount), mint_entry.utxo.record.amount.get_amount());
        }
        Ok(())
    };

    judgement_result(result, vec![100, 900])
}

#[test]
fn test() {
    pnk!(test_query_coinbase_hist());
}
