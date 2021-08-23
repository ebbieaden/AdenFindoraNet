use crate::{
    abci::{server::ABCISubmissionServer, staking},
    api::{query_server::BLOCK_CREATED, submission_server::convert_tx},
};
use fp_storage::hash::StorageHasher;
use lazy_static::lazy_static;
use ledger::{address::is_convert_tx, data_model::TxnEffect, staking::is_coinbase_tx};
use log::debug;
use parking_lot::Mutex;
use ruc::*;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};
use tendermint_sys::SyncApplication;
use tm_protos::abci::*;

mod pulse_cache;
mod utils;

/// current block height
pub static TENDERMINT_BLOCK_HEIGHT: AtomicI64 = AtomicI64::new(0);

lazy_static! {
    static ref REQ_BEGIN_BLOCK: Arc<Mutex<RequestBeginBlock>> =
        Arc::new(Mutex::new(RequestBeginBlock::default()));
}

pub fn info(s: &mut ABCISubmissionServer, req: RequestInfo) -> ResponseInfo {
    let mut resp = ResponseInfo::default();
    let resp_app = s.account_base_app.write().info(req);

    let mut la = s.la.write();

    let mut state = la.get_committed_state().write();
    let commitment = state.get_state_commitment();
    if commitment.1 > 0 {
        // last height
        let td_height = resp_app.last_block_height;
        resp.last_block_height = td_height;

        // last hash
        let la_hash = commitment.0.as_ref().to_vec();
        let cs_hash = resp_app.last_block_app_hash;
        resp.last_block_app_hash = root_hash("info", td_height, la_hash, cs_hash);
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

pub fn query(s: &mut ABCISubmissionServer, req: RequestQuery) -> ResponseQuery {
    s.account_base_app.write().query(req)
}

pub fn init_chain(
    s: &mut ABCISubmissionServer,
    req: RequestInitChain,
) -> ResponseInitChain {
    s.account_base_app.write().init_chain(req)
}

pub fn check_tx(s: &mut ABCISubmissionServer, req: RequestCheckTx) -> ResponseCheckTx {
    if let Some(tx) = convert_tx(&req.tx) {
        // Get the Tx [u8] and convert to u64
        let mut resp = ResponseCheckTx::default();
        if is_convert_tx(&tx) {
            let check_res = s.account_base_app.write().check_findora_tx(&tx);
            if check_res.is_err() {
                resp.code = 1;
                resp.log = String::from("Check, failed");
            }
        } else if is_coinbase_tx(&tx)
            || !tx.is_basic_valid(TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed))
            || ruc::info!(TxnEffect::compute_effect(tx)).is_err()
        {
            resp.code = 1;
            resp.log = String::from("Check failed");
        }
        resp
    } else {
        s.account_base_app.write().check_tx(req)
    }
}

pub fn deliver_tx(
    s: &mut ABCISubmissionServer,
    req: RequestDeliverTx,
) -> ResponseDeliverTx {
    if let Some(tx) = convert_tx(&req.tx) {
        let mut resp = ResponseDeliverTx::default();
        if !is_coinbase_tx(&tx)
            && tx.is_basic_valid(TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed))
        {
            // set attr(tags) if any
            let attr = utils::gen_tendermint_attr(&tx);
            if !attr.is_empty() {
                resp.events = attr;
            }

            if s.la.write().cache_transaction(tx.clone()).is_ok() {
                if is_convert_tx(&tx)
                    && s.account_base_app.write().deliver_findora_tx(&tx).is_err()
                {
                    resp.code = 1;
                    resp.log = String::from("Failed to deliver transaction!");
                }
                return resp;
            }
        }
        resp.code = 1;
        resp.log = String::from("Failed to deliver transaction!");
        resp
    } else {
        s.account_base_app.write().deliver_tx(req)
    }
}

pub fn begin_block(
    s: &mut ABCISubmissionServer,
    req: RequestBeginBlock,
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
    drop(la);

    s.account_base_app.write().begin_block(req)
}

pub fn end_block(
    s: &mut ABCISubmissionServer,
    req: RequestEndBlock,
) -> ResponseEndBlock {
    let mut resp = ResponseEndBlock::default();

    let begin_block_req = REQ_BEGIN_BLOCK.lock();
    let header = pnk!(begin_block_req.header.as_ref());

    let mut la = s.la.write();

    // mint coinbase, cache system transactions to ledger
    {
        let laa = la.get_committed_state().write();
        if let Some(tx) =
            staking::system_mint_pay(&*laa, &mut *s.account_base_app.write())
        {
            drop(laa);
            // this unwrap should be safe
            la.cache_transaction(tx).unwrap();
        }
    }

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

    if let Ok(Some(vs)) = ruc::info!(staking::get_validators(
        la.get_committed_state().read().get_staking(),
        begin_block_req.last_commit_info.as_ref()
    )) {
        resp.validator_updates = vs;
    }

    staking::system_ops(
        &mut *la.get_committed_state().write(),
        &header,
        begin_block_req.last_commit_info.as_ref(),
        &begin_block_req.byzantine_validators.as_slice(),
    );

    let _ = s.account_base_app.write().end_block(req);

    resp
}

pub fn commit(s: &mut ABCISubmissionServer) -> ResponseCommit {
    let mut r = ResponseCommit::default();
    let la = s.la.read();

    // la.begin_commit();

    let state = la.get_committed_state().read();
    let commitment = state.get_state_commitment();

    // la.end_commit();

    let td_height = TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed);
    pnk!(pulse_cache::write_height(td_height));

    pnk!(pulse_cache::write_staking(state.get_staking()));

    pnk!(pulse_cache::write_block_pulse(la.block_pulse_count()));

    // set root hash
    let la_hash = commitment.0.as_ref().to_vec();
    let cs_hash = s.account_base_app.write().commit().data;
    r.data = root_hash("commit", td_height, la_hash, cs_hash);

    r
}

/// Combines ledger state hash and chain state hash
fn root_hash(
    tag: &str,
    height: i64,
    mut la_hash: Vec<u8>,
    mut cs_hash: Vec<u8>,
) -> Vec<u8> {
    debug!(
        "root_hash_{}: {}_{}, height: {}",
        tag,
        hex::encode(la_hash.clone()),
        hex::encode(cs_hash.clone()),
        height
    );
    la_hash.append(&mut cs_hash);
    fp_storage::hash::Sha256::hash(la_hash.as_slice()).to_vec()
}
