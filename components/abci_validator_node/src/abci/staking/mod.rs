//!
//! # Staking
//!
//! Business logic based on [**Ledger Staking**](ledger::staking).
//!

use crate::abci::server::forward_txn_with_mode;
use abci::{Evidence, Header, LastCommitInfo, PubKey, ValidatorUpdate};
use lazy_static::lazy_static;
use ledger::{
    data_model::{Transaction, TransferType, TxoRef, TxoSID, Utxo, ASSET_TYPE_FRA},
    staking::{
        ops::governance::{governance_penalty_tendermint_auto, ByzantineKind},
        Staking,
    },
    store::{LedgerAccess, LedgerUpdate},
};
use rand_core::{CryptoRng, RngCore};
use ruc::*;
use std::{collections::HashMap, env};
use txn_builder::TransferOperationBuilder;
use zei::xfr::asset_record::{open_blind_asset_record, AssetRecordType};
use zei::xfr::{
    sig::XfrPublicKey,
    structs::{AssetRecordTemplate, XfrAmount, XfrAssetType},
};

type SignedPower = i64;

// The top 50 candidate validators
// will become official validators.
const VALIDATOR_LIMIT: usize = 50;

lazy_static! {
    /// Tendermint node address, sha256(pubkey)[:20]
    pub static ref TD_NODE_SELF_ADDR: Vec<u8> = {
        let hex_addr = pnk!(env::var("TD_NODE_SELF_ADDR"));
        let bytes_addr = pnk!(hex::decode(hex_addr));
        assert_eq!(20, bytes_addr.len());
        bytes_addr
    };
}

/// Get the effective validators at current block height.
pub fn get_validators(staking: &Staking) -> Result<Vec<ValidatorUpdate>> {
    let mut vs = staking
        .validator_get_current()
        .c(d!())?
        .body
        .values()
        .map(|v| (v.td_power, &v.td_pubkey))
        .collect::<Vec<_>>();

    // Ensure the minimal amount of BFT-like algorithm
    if 3 > vs.len() {
        return Err(eg!("invalid settings"));
    }

    // reverse sort
    vs.sort_by_key(|v| -v.0);

    Ok(vs
        .iter()
        .take(VALIDATOR_LIMIT)
        .map(|(power, pubkey)| {
            let mut vu = ValidatorUpdate::new();
            let mut pk = PubKey::new();
            // pk.set_field_type("ed25519".to_owned());
            pk.set_data(pubkey.to_vec());
            vu.set_power(*power);
            vu.set_pub_key(pk);
            vu
        })
        .collect())
}

// Call this function in `BeginBlock`,
// - pay delegation rewards
// - pay proposer rewards(traditional block rewards)
// - do governance operations
pub fn system_ops<RNG: RngCore + CryptoRng>(
    la: &mut (impl LedgerAccess + LedgerUpdate<RNG>),
    header: &Header,
    last_commit_info: Option<&LastCommitInfo>,
    evs: &[Evidence],
    fwder: &str,
) {
    ruc::info_omit!(system_pay(la, &header.proposer_address, fwder));

    let staking = la.get_staking_mut();
    ruc::info_omit!(set_rewards(
        staking,
        &header.proposer_address,
        last_commit_info.map(|lci| get_last_vote_power(lci))
    ));

    evs.iter()
        .filter(|ev| ev.validator.is_some())
        .for_each(|ev| {
            let v = ev.validator.as_ref().unwrap();
            let bz = ByzantineInfo {
                addr: &hex::encode(&v.address),
                kind: ev.field_type.as_str(),
                height: ev.height,
                timestamp: ev.time.as_ref().map(|ts| ts.seconds).unwrap_or(0),
                power: v.power,
                total_power: ev.total_voting_power,
            };

            ruc::info_omit!(system_governance(staking, &bz));
        });
}

// Get the actual total power of last block.
fn get_last_vote_power(last_commit_info: &LastCommitInfo) -> SignedPower {
    last_commit_info
        .votes
        .iter()
        .filter(|v| v.signed_last_block)
        .flat_map(|info| info.validator.as_ref().map(|v| v.power))
        .sum()
}

// Set delegation rewards and proposer rewards
fn set_rewards(
    staking: &mut Staking,
    proposer: &[u8],
    last_vote_power: Option<i64>,
) -> Result<()> {
    staking
        .set_last_block_rewards(&hex::encode(proposer), last_vote_power)
        .c(d!())
}

#[allow(dead_code)]
struct ByzantineInfo<'a> {
    addr: &'a str,
    // - "UNKNOWN"
    // - "DUPLICATE_VOTE"
    // - "LIGHT_CLIENT_ATTACK"
    kind: &'a str,
    height: i64,
    timestamp: i64,
    power: i64,
    total_power: i64,
}

// Auto governance.
fn system_governance(staking: &mut Staking, bz: &ByzantineInfo) -> Result<()> {
    let kind = match bz.kind {
        "DUPLICATE_VOTE" => ByzantineKind::DuplicateVote,
        "LIGHT_CLIENT_ATTACK" => ByzantineKind::LightClientAttack,
        "UNKNOWN" => ByzantineKind::Unknown,
        _ => return Err(eg!()),
    };
    governance_penalty_tendermint_auto(staking, bz.addr, &kind).c(d!())
}

// Pay for bond 'Delegations' and 'FraDistributions'.
fn system_pay(la: &impl LedgerAccess, proposer: &[u8], fwder: &str) -> Result<()> {
    if *TD_NODE_SELF_ADDR != proposer {
        return Ok(());
    }

    let staking = la.get_staking();

    // at most 256 items to pay per block
    let mut paylist = staking
        .delegation_get_rewards()
        .into_iter()
        .chain(
            staking
                .fra_distribution_get_plan()
                .iter()
                .map(|(k, v)| (*k, *v)),
        )
        .take(256)
        .collect::<Vec<_>>();

    if paylist.is_empty() {
        return Ok(());
    }

    // sort by amount
    paylist.sort_by_key(|i| i.1);

    let coinbase_utxo_sids = staking.coinbase_txos().into_iter().collect::<Vec<_>>();

    gen_transaction(la, coinbase_utxo_sids, paylist)
        .c(d!())
        .and_then(|tx| forward_txn_with_mode(fwder, tx, true).c(d!()))
}

// Generate all tx in batch mode.
fn gen_transaction(
    la: &impl LedgerAccess,
    utxo_sids: Vec<TxoSID>,
    paylist: Vec<(XfrPublicKey, u64)>,
) -> Result<Transaction> {
    let staking = la.get_staking();
    let seq_id = la.get_state_commitment().1;

    let mut inputs = vec![];
    let mut outputs = map! {};

    for i in 0..paylist.len() {
        let mut total_amount =
            paylist.iter().rev().skip(i).map(|(_, am)| *am).sum::<u64>();
        let mut sids = utxo_sids.iter();
        while total_amount > 0 {
            if let Some(sid) = sids.next() {
                if let Some(auth_utxo) = la.get_utxo(*sid) {
                    let utxo = auth_utxo.utxo;
                    if let XfrAssetType::NonConfidential(ty) = utxo.0.record.asset_type {
                        if ASSET_TYPE_FRA == ty {
                            if let XfrAmount::NonConfidential(am) = utxo.0.record.amount
                            {
                                inputs.push((
                                    *sid,
                                    utxo,
                                    alt!(total_amount > am, am, total_amount),
                                ));
                                total_amount = total_amount.saturating_sub(am);
                            }
                        }
                    }
                }
            } else {
                // insufficient balance in coinbase
                break;
            }
        }

        if 0 == total_amount {
            outputs = paylist.into_iter().rev().skip(i).collect();
            break;
        }
    }

    alt!(inputs.is_empty() || outputs.is_empty(), return Err(eg!()));

    do_gen_transaction(staking, inputs, outputs, seq_id).c(d!())
}

// **NOTE:**
// transfer from CoinBase need not to pay FEE
fn do_gen_transaction(
    staking: &Staking,
    inputs: Vec<(TxoSID, Utxo, u64)>,
    dests: HashMap<XfrPublicKey, u64>,
    seq_id: u64,
) -> Result<Transaction> {
    let keypair = staking.coinbase_keypair();

    let mut op = TransferOperationBuilder::new();

    for (sid, utxo, n) in inputs.into_iter() {
        op.add_input(
            TxoRef::Absolute(sid),
            open_blind_asset_record(&utxo.0.record, &None, keypair).unwrap(),
            None,
            None,
            n,
        )
        .c(d!())?;
    }

    for output in dests.iter().map(|(pk, n)| {
        AssetRecordTemplate::with_no_asset_tracing(
            *n,
            ASSET_TYPE_FRA,
            AssetRecordType::NonConfidentialAmount_NonConfidentialAssetType,
            *pk,
        )
    }) {
        op.add_output(&output, None, None, None).c(d!())?;
    }

    let op = op
        .balance()
        .c(d!())?
        .create(TransferType::Standard)
        .c(d!())?
        .sign(keypair)
        .c(d!())?
        .transaction()
        .c(d!())?;

    Ok(Transaction::from_operation(op, seq_id))
}

#[cfg(test)]
#[cfg(feature = "abci_mock")]
pub mod abci_mock_test;
