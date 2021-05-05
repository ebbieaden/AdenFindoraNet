use super::staking;
use abci::*;
use lazy_static::lazy_static;
use ledger::data_model::{Operation, Transaction, TxnEffect, TxnSID};
use ledger::store::*;
use parking_lot::RwLock;
use protobuf::RepeatedField;
use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;
use ruc::*;
use serde::Serialize;
use std::env;
use std::path::Path;
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};
use submission_server::{convert_tx, SubmissionServer};
use zei::xfr::structs::{XfrAmount, XfrAssetType};

#[cfg(feature = "abci_mock")]
use abci_mock_tx_sender::TendermintForward;
#[cfg(not(feature = "abci_mock"))]
use tx_sender::TendermintForward;

#[cfg(feature = "abci_mock")]
mod abci_mock_tx_sender;
mod pulse_cache;
#[cfg(not(feature = "abci_mock"))]
mod tx_sender;

#[cfg(feature = "abci_mock")]
pub use abci_mock_tx_sender::forward_txn_with_mode;
#[cfg(not(feature = "abci_mock"))]
pub use tx_sender::forward_txn_with_mode;

static TENDERMINT_BLOCK_HEIGHT: AtomicI64 = AtomicI64::new(0);

lazy_static! {
    /// Tendermint node address, sha256(pubkey)[:20]
    pub static ref TD_NODE_SELF_ADDR: Vec<u8> = {
        let hex_addr = pnk!(env::var("TD_NODE_SELF_ADDR"));
        let bytes_addr = pnk!(hex::decode(hex_addr));
        assert_eq!(20, bytes_addr.len());
        bytes_addr
    };
}

pub struct ABCISubmissionServer {
    pub la: Arc<RwLock<SubmissionServer<ChaChaRng, LedgerState, TendermintForward>>>,
}

impl ABCISubmissionServer {
    pub fn new(
        base_dir: Option<&Path>,
        tendermint_reply: String,
    ) -> Result<ABCISubmissionServer> {
        let ledger_state = match base_dir {
            None => LedgerState::test_ledger(),
            Some(base_dir) => pnk!(LedgerState::load_or_init(base_dir)),
        };
        let prng = rand_chacha::ChaChaRng::from_entropy();
        Ok(ABCISubmissionServer {
            la: Arc::new(RwLock::new(
                SubmissionServer::new_no_auto_commit(
                    prng,
                    Arc::new(RwLock::new(ledger_state)),
                    Some(TendermintForward { tendermint_reply }),
                )
                .c(d!())?,
            )),
        })
    }
}

// TODO: implement abci hooks
impl abci::Application for ABCISubmissionServer {
    fn info(&mut self, _req: &RequestInfo) -> ResponseInfo {
        let mut resp = ResponseInfo::new();
        {
            let la = self.la.read();
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

    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
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

    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        let mut resp = ResponseDeliverTx::new();
        if let Some(tx) = convert_tx(req.get_tx()) {
            if tx.is_basic_valid(TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed)) {
                // set attr(tags) if any
                let attr = gen_tendermint_attr(&tx);
                if 0 < attr.len() {
                    resp.set_events(attr);
                }

                if self.la.write().cache_transaction(tx).is_ok() {
                    return resp;
                }
            }
        }

        resp.set_code(1);
        resp.set_log(String::from("Failed to deliver transaction!"));
        resp
    }

    fn begin_block(&mut self, req: &RequestBeginBlock) -> ResponseBeginBlock {
        let header = pnk!(req.header.as_ref());
        TENDERMINT_BLOCK_HEIGHT.swap(header.height, Ordering::Relaxed);

        {
            let mut la = self.la.write();

            staking::system_ops(
                &mut *la.get_committed_state().write(),
                &header,
                req.last_commit_info.as_ref(),
                &req.byzantine_validators.as_slice(),
                la.get_fwder().unwrap().as_ref(),
            );

            if la.all_commited() {
                la.begin_block();
            }
        }

        ResponseBeginBlock::new()
    }

    fn end_block(&mut self, _req: &RequestEndBlock) -> ResponseEndBlock {
        let mut la = self.la.write();
        if la.block_txn_count() == 0 {
            la.pulse_block();
        } else if !la.all_commited() {
            if let Err(e) = la.end_block().c(d!()) {
                e.print();
            }
        }

        let mut resp = ResponseEndBlock::new();
        if let Ok(vs) = ruc::info!(staking::get_validators(
            la.get_committed_state().read().get_staking()
        )) {
            resp.set_validator_updates(RepeatedField::from_vec(vs));
        }
        resp
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        let mut r = ResponseCommit::new();
        {
            let la = self.la.read();
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
}

/////////////////////////////////////////////////////////////////////////////////

/// generate attr(tags) for index-ops of tendermint
///   - "tx.exist" => "y"
///   - "addr.from" => "Json<TagAttr>"
///   - "addr.to" => "Json<TagAttr>"
///   - "addr.from.<addr>" => "y"
///   - "addr.to.<addr>" => "y"
fn gen_tendermint_attr(tx: &Transaction) -> RepeatedField<Event> {
    let mut res = vec![];

    // index txs without block info
    let mut ev = Event::new();
    ev.set_field_type("tx".to_owned());

    let mut kv = vec![Pair::new(), Pair::new()];
    kv[0].set_key("prehash".as_bytes().to_vec());
    kv[0].set_value(hex::encode(tx.hash(TxnSID(0))).into_bytes());
    kv[1].set_key("timestamp".as_bytes().to_vec());
    kv[1].set_value(
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string()
            .into_bytes(),
    );

    ev.set_attributes(RepeatedField::from_vec(kv));
    res.push(ev);

    let (from, to) = gen_tendermint_attr_addr(tx);

    if !from.is_empty() || !to.is_empty() {
        let mut ev = Event::new();
        ev.set_field_type("addr".to_owned());

        let mut kv = vec![Pair::new(), Pair::new()];
        kv[0].set_key("from".as_bytes().to_vec());
        kv[0].set_value(serde_json::to_vec(&from).unwrap());
        kv[1].set_key("to".as_bytes().to_vec());
        kv[1].set_value(serde_json::to_vec(&to).unwrap());

        ev.set_attributes(RepeatedField::from_vec(kv));
        res.push(ev);

        macro_rules! index_addr {
            ($attr: expr, $ty: expr) => {
                let kv = $attr
                    .into_iter()
                    .map(|i| {
                        let mut p = Pair::new();
                        p.set_key(i.addr.into_bytes());
                        p.set_value("y".as_bytes().to_vec());
                        p
                    })
                    .collect::<Vec<_>>();

                if !kv.is_empty() {
                    let mut ev = Event::new();
                    ev.set_field_type($ty.to_owned());
                    ev.set_attributes(RepeatedField::from_vec(kv));
                    res.push(ev);
                }
            };
        }

        index_addr!(from, "addr.from");
        index_addr!(to, "addr.to");
    }

    RepeatedField::from_vec(res)
}

// collect informations of inputs and outputs
// # return: ([from ...], [to ...])
fn gen_tendermint_attr_addr(tx: &Transaction) -> (Vec<TagAttr>, Vec<TagAttr>) {
    tx.body
        .operations
        .iter()
        .fold((vec![], vec![]), |mut base, new| {
            macro_rules! append_attr {
                // trasfer\bind\release
                ($data: expr, $direction: tt, $idx: tt) => {
                    $data.body.transfer.$direction.iter().for_each(|i| {
                        let mut attr = TagAttr::default();
                        attr.addr = wallet::public_key_to_bech32(&i.public_key);
                        if let XfrAssetType::NonConfidential(ty) = i.asset_type {
                            attr.asset_type = Some(hex::encode(&ty.0[..]));
                        }
                        if let XfrAmount::NonConfidential(am) = i.amount {
                            attr.asset_amount = Some(am);
                        }
                        base.$idx.push(attr);
                    });
                };
                // define\issue\AIR\memo
                ($data: expr) => {
                    let mut attr = TagAttr::default();
                    attr.addr = wallet::public_key_to_bech32(&$data.pubkey);
                    base.0.push(attr);
                };
            }

            match new {
                Operation::TransferAsset(d) => {
                    append_attr!(d, inputs, 0);
                    append_attr!(d, outputs, 1);
                }
                Operation::DefineAsset(d) => {
                    append_attr!(d);
                }
                Operation::IssueAsset(d) => {
                    append_attr!(d);
                }
                Operation::UpdateMemo(d) => {
                    append_attr!(d);
                }
                _ => {}
            }

            base
        })
}

#[derive(Serialize, Default)]
struct TagAttr {
    // FRA address
    addr: String,
    // hex.encode(asset_type)
    asset_type: Option<String>,
    asset_amount: Option<u64>,
}
