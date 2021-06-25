use crate::abci::{server::ABCISubmissionServer, staking};
use abci::*;
use fp_storage::hash::StorageHasher;
use lazy_static::lazy_static;
use ledger::{
    data_model::TxnEffect,
    store::{LedgerAccess, LedgerUpdate},
};
use parking_lot::Mutex;
use protobuf::RepeatedField;
use query_server::BLOCK_CREATED;
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

    let mut la = s.la.write();

    let mut state = la.get_committed_state().write();
    let commitment = state.get_state_commitment();
    if commitment.1 > 0 {
        let tendermint_height = commitment.1 + state.get_pulse_count();
        resp.set_last_block_height(tendermint_height as i64);
        resp.set_last_block_app_hash(commitment.0.as_ref().to_vec());
    }

    if let Ok(h) = ruc::info!(pulse_cache::read_height()) {
        resp.set_last_block_height(h);
    }

    if let Ok(s) = ruc::info!(pulse_cache::read_staking()) {
        *state.get_staking_mut() = s;
    }

    if let Ok(cnt) = ruc::info!(pulse_cache::read_block_pulse()) {
        drop(state);
        if la.all_commited() {
            la.begin_block();
        }
        la.restore_block_pulse(cnt);
    }

    resp
}

pub fn query(s: &mut ABCISubmissionServer, req: &RequestQuery) -> ResponseQuery {
    s.app.query(req)
}

pub fn init_chain(
    s: &mut ABCISubmissionServer,
    req: &RequestInitChain,
) -> ResponseInitChain {
    s.app.init_chain(req)
}

pub fn check_tx(s: &mut ABCISubmissionServer, req: &RequestCheckTx) -> ResponseCheckTx {
    // Get the Tx [u8] and convert to u64
    if let Some(tx) = convert_tx(req.get_tx()) {
        let mut resp = ResponseCheckTx::new();
        if !tx.is_basic_valid(TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed))
            || ruc::info!(TxnEffect::compute_effect(tx)).is_err()
        {
            resp.set_code(1);
            resp.set_log(String::from("Check failed"));
        }
        resp
    } else {
        s.app.check_tx(req)
    }
}

pub fn deliver_tx(
    s: &mut ABCISubmissionServer,
    req: &RequestDeliverTx,
) -> ResponseDeliverTx {
    if let Some(tx) = convert_tx(req.get_tx()) {
        let mut resp = ResponseDeliverTx::new();
        if tx.is_basic_valid(TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed)) {
            // set attr(tags) if any
            let attr = utils::gen_tendermint_attr(&tx);
            if 0 < attr.len() {
                resp.set_events(attr);
            }

            if s.address_binder.deliver_tx(&tx).is_ok()
               && s.la.write().cache_transaction(tx).is_ok()
            {
                return resp;
            }
        }
        resp.set_code(1);
        resp.set_log(String::from("Failed to deliver transaction!"));
        resp
    } else {
        s.app.deliver_tx(req)
    }
}

pub fn begin_block(
    s: &mut ABCISubmissionServer,
    req: &RequestBeginBlock,
) -> ResponseBeginBlock {
    let header = pnk!(req.header.as_ref());
    TENDERMINT_BLOCK_HEIGHT.swap(header.height, Ordering::Relaxed);

    *REQ_BEGIN_BLOCK.lock() = req.clone();

    let mut la = s.la.write();

    // set height first
    la.get_committed_state()
        .write()
        .get_staking_mut()
        .set_custom_block_height(header.height as u64);

    // then create new block or update simulator
    if la.all_commited() {
        la.begin_block();
    } else {
        pnk!(la.update_staking_simulator());
    }

    s.app.begin_block(req)
}

pub fn end_block(
    s: &mut ABCISubmissionServer,
    req: &RequestEndBlock,
) -> ResponseEndBlock {
    let mut resp = ResponseEndBlock::new();

    let mut la = s.la.write();

    if la.block_txn_count() == 0 {
        la.pulse_block();
    } else if !la.all_commited() {
        pnk!(la.end_block());

        {
            let mut created = BLOCK_CREATED.0.lock();
            *created = true;
            BLOCK_CREATED.1.notify_one();
        }
    }

    let begin_block_req = REQ_BEGIN_BLOCK.lock();
    let header = pnk!(begin_block_req.header.as_ref());

    let is_replaying = !begin_block_req.appHashCurReplay.is_empty();

    if !is_replaying {
        if let Ok(Some(vs)) = ruc::info!(staking::get_validators(
            la.get_committed_state().read().get_staking(),
            begin_block_req.last_commit_info.as_ref(),
        )) {
            resp.set_validator_updates(RepeatedField::from_vec(vs));
        }
    }

    staking::system_ops(
        &mut *la.get_committed_state().write(),
        &header,
        begin_block_req.last_commit_info.as_ref(),
        &begin_block_req.byzantine_validators.as_slice(),
        la.get_fwder().as_ref(),
        is_replaying,
    );

    s.app.end_block(req);

    resp
}

pub fn commit(s: &mut ABCISubmissionServer, req: &RequestCommit) -> ResponseCommit {
    let mut r = ResponseCommit::new();
    let la = s.la.read();

    // la.begin_commit();

    let state = la.get_committed_state().read();
    let commitment = state.get_state_commitment();

    // la.end_commit();

    // r.set_data(commitment.0.as_ref().to_vec());

    pnk!(pulse_cache::write_height(
        TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed)
    ));

    pnk!(pulse_cache::write_staking(state.get_staking()));

    pnk!(pulse_cache::write_block_pulse(la.block_pulse_count()));

    let mut la_hash = commitment.0.as_ref().to_vec();
    let mut cs_hash = s.app.commit(req).data;
    la_hash.append(&mut cs_hash);
    r.set_data(fp_storage::hash::Sha256::hash(la_hash.as_slice()).to_vec());

    r
}
