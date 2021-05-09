//!
//! # Integration Testing
//!
//! The content of on-chain governance is not covered.
//!

use crate::abci::server::ABCISubmissionServer;
use abci::*;
use lazy_static::lazy_static;
use ledger::{
    data_model::{Transaction, TxoSID, Utxo, BLACK_HOLE_PUBKEY},
    staking::td_pubkey_to_td_addr,
};
use parking_lot::RwLock;
use ruc::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};
use zei::xfr::sig::XfrPublicKey;

lazy_static! {
    static ref ABCI_MOCKER: Arc<RwLock<AbciMocker>> = Arc::new(RwLock::new(AbciMocker::new()));
    /// will be used in [tx_sender](super::server::tx_sender)
    pub static ref TD_MOCKER: TendermintMocker = TendermintMocker::new();
}

static TENDERMINT_BLOCK_HEIGHT: AtomicI64 = AtomicI64::new(0);

struct AbciMocker(ABCISubmissionServer);

impl AbciMocker {
    fn new() -> AbciMocker {
        AbciMocker(pnk!(ABCISubmissionServer::new(None, String::new())))
    }

    fn produce_block(&mut self) {
        let h = 1 + TENDERMINT_BLOCK_HEIGHT.fetch_add(1, Ordering::Relaxed);
        let proposer = TD_MOCKER.validators.read().keys().next().unwrap().to_vec();

        self.0.begin_block(&gen_req_begin_block(h, proposer));

        for tx in TD_MOCKER.mem_pool.try_iter() {
            self.0.deliver_tx(&gen_req_deliver_tx(tx));
        }

        let resp = self.0.end_block(&gen_req_end_block());
        if 0 < resp.validator_updates.len() {
            *TD_MOCKER.validators.write() = resp
                .validator_updates
                .into_vec()
                .into_iter()
                .filter_map(|v| {
                    v.pub_key
                        .as_ref()
                        .map(|pk| (td_pubkey_to_td_addr(pk.get_data()), v.power))
                })
                .collect();
        }

        self.0.commit(&gen_req_commit());
    }

    fn get_owned_utxos(&self, addr: &XfrPublicKey) -> HashMap<TxoSID, Utxo> {
        self.0
            .la
            .read()
            .get_committed_state()
            .read()
            .get_status()
            .get_owned_utxos(addr)
    }
}

pub struct TendermintMocker {
    pub block_itv: u64,
    mem_pool: Receiver<Transaction>,
    pub sender: Sender<Transaction>,
    validators: Arc<RwLock<HashMap<Vec<u8>, i64>>>,
}

unsafe impl Send for TendermintMocker {}
unsafe impl Sync for TendermintMocker {}

impl TendermintMocker {
    fn new() -> TendermintMocker {
        let itv = 100;

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(itv));
                ABCI_MOCKER.write().produce_block();
            }
        });

        let (sender, recver) = channel();
        let validators = Arc::new(RwLock::new(map! { [0; 20].to_vec() => 1 }));

        TendermintMocker {
            block_itv: itv,
            mem_pool: recver,
            sender,
            validators,
        }
    }

    fn clean(&self) {
        self.mem_pool.try_iter().for_each(|_| {});
        *self.validators.write() = map! { [0; 20].to_vec() => 1 };
    }
}

fn gen_req_begin_block(h: i64, proposer: Vec<u8>) -> RequestBeginBlock {
    let mut header = Header::new();
    header.set_height(h);
    header.set_proposer_address(proposer);

    let mut res = RequestBeginBlock::new();
    res.set_header(header);

    res
}

fn gen_req_deliver_tx(tx: Transaction) -> RequestDeliverTx {
    let mut res = RequestDeliverTx::new();
    res.set_tx(pnk!(serde_json::to_vec(&tx)));
    res
}

fn gen_req_end_block() -> RequestEndBlock {
    RequestEndBlock::new()
}

fn gen_req_commit() -> RequestCommit {
    RequestCommit::new()
}

fn env_refresh() {
    *ABCI_MOCKER.write() = AbciMocker::new();
    TD_MOCKER.clean();
}

// 0. issue FRA
// 1. paid 400m FRAs to CoinBase
// 2. transfer some FRAs to a new addr `X`
// 3. use `X` to propose a delegation(block span = 10)
// 4. ensure `X` can not do transfer within block span
// 5. ensure the power of co-responding validator is increased
// 6. wait for the end of bond state
// 7. ensure the power of co-responding validator is decreased
// 8. ensure delegation reward is calculated and paid correctly
// 9. ensure `X` can do transfer after bond-state expired
//
// 10. transfer some FRAs to `X`
// 11. use `X` to propose a delegation(block span = 10_0000)
// 12. ensure `X` can not do transfer within block span
// 13. ensure the power of co-responding validator is increased
// 14  propose a `UnDelegation` tx to force end the delegation
// 15. ensure the power of co-responding validator is decreased
// 16. ensure delegation reward is calculated and paid correctly
// 17. ensure `X` can do transfer after bond-state expired
//
// 18. try to transfer FRAs from CoinBase
// with invalid amount or target addr, and ensure it will fail
//
// 19. update validators
// 20. use `FraDistribution` to transfer FRAs to multi addrs
// 21. ensure the result of `FraDistribution` is correct
// 22. use these addrs to delegate to different validators(block span = 10)
// 23. ensure the power of each validator is increased correctly
// 24. wait for the end of bond state
// 25. ensure the power of each validator is decreased correctly
// 26. ensure delegation-rewards-rate is correct in different global delegation levels
#[test]
fn staking_integration() {
    env_refresh();
    ABCI_MOCKER.read().get_owned_utxos(&BLACK_HOLE_PUBKEY);

    // TODO
}
