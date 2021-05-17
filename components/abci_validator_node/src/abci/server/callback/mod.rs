use crate::abci::{server::ABCISubmissionServer, staking};
use abci::*;
use lazy_static::lazy_static;
use ledger::{data_model::TxnEffect, store::LedgerAccess};
use parking_lot::Mutex;
use protobuf::RepeatedField;
use ruc::*;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};
use submission_server::convert_tx;

mod pulse_cache;
mod utils;

/// current block height
pub static TENDERMINT_BLOCK_HEIGHT: AtomicI64 = AtomicI64::new(0);

lazy_static! {
    static ref REQ_BEGIN_BLOCK: Arc<Mutex<RequestBeginBlock>> =
        Arc::new(Mutex::new(RequestBeginBlock::new()));
}

pub fn info(s: &mut ABCISubmissionServer, _req: &RequestInfo) -> ResponseInfo {
    let mut resp = ResponseInfo::new();
    {
        let la = s.la.read();
        let state = la.get_committed_state().read();
        let commitment = state.get_state_commitment();
        if commitment.1 > 0 {
            let tendermint_height = commitment.1 + state.get_pulse_count();
            resp.set_last_block_height(tendermint_height as i64);
            resp.set_last_block_app_hash(commitment.0.as_ref().to_vec());
        }

        if let Ok(h) = ruc::info!(pulse_cache::read_height()) {
            resp.set_last_block_height(h);
        }
    }

    resp
}

pub fn check_tx(_s: &mut ABCISubmissionServer, req: &RequestCheckTx) -> ResponseCheckTx {
    // Get the Tx [u8] and convert to u64
    let mut resp = ResponseCheckTx::new();

    if let Some(tx) = convert_tx(req.get_tx()) {
        if !tx.is_basic_valid(TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed))
            || ruc::info!(TxnEffect::compute_effect(tx)).is_err()
        {
            resp.set_code(1);
            resp.set_log(String::from("Check failed"));
        }
    } else {
        resp.set_code(1);
        resp.set_log(String::from("Could not unpack transaction"));
    }

    resp
}

pub fn deliver_tx(
    s: &mut ABCISubmissionServer,
    req: &RequestDeliverTx,
) -> ResponseDeliverTx {
    let mut resp = ResponseDeliverTx::new();
    if let Some(tx) = convert_tx(req.get_tx()) {
        if tx.is_basic_valid(TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed)) {
            // set attr(tags) if any
            let attr = utils::gen_tendermint_attr(&tx);
            if 0 < attr.len() {
                resp.set_events(attr);
            }

            if s.la.write().cache_transaction(tx).is_ok() {
                return resp;
            }
        }
    }

    resp.set_code(1);
    resp.set_log(String::from("Failed to deliver transaction!"));
    resp
}

pub fn begin_block(
    s: &mut ABCISubmissionServer,
    req: &RequestBeginBlock,
) -> ResponseBeginBlock {
    let header = pnk!(req.header.as_ref());
    TENDERMINT_BLOCK_HEIGHT.swap(header.height, Ordering::Relaxed);

    *REQ_BEGIN_BLOCK.lock() = req.clone();

    let mut la = s.la.write();

    if la.all_commited() {
        la.begin_block();
    }

    ResponseBeginBlock::new()
}

pub fn end_block(
    s: &mut ABCISubmissionServer,
    _req: &RequestEndBlock,
) -> ResponseEndBlock {
    let mut la = s.la.write();
    if la.block_txn_count() == 0 {
        la.pulse_block();
    } else if !la.all_commited() {
        if let Err(e) = la.end_block().c(d!()) {
            e.print();
        }
    }

    let mut resp = ResponseEndBlock::new();
    let begin_block_req = REQ_BEGIN_BLOCK.lock();
    let header = pnk!(begin_block_req.header.as_ref());

    if let Ok(Some(vs)) = ruc::info!(staking::get_validators(
        la.get_committed_state().read().get_staking(),
        begin_block_req.last_commit_info.as_ref()
    )) {
        resp.set_validator_updates(RepeatedField::from_vec(vs));
    }

    staking::system_ops(
        &mut *la.get_committed_state().write(),
        &header,
        begin_block_req.last_commit_info.as_ref(),
        &begin_block_req.byzantine_validators.as_slice(),
        la.get_fwder().unwrap().as_ref(),
    );

    resp
}

pub fn commit(s: &mut ABCISubmissionServer, _req: &RequestCommit) -> ResponseCommit {
    let mut r = ResponseCommit::new();
    {
        let la = s.la.read();
        // la.begin_commit();
        let commitment = la.get_committed_state().read().get_state_commitment();
        // la.end_commit();
        r.set_data(commitment.0.as_ref().to_vec());
    }

    pnk!(pulse_cache::write_height(
        TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed)
    ));

    r
}
