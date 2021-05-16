//!
//! # Staking
//!
//! Business logic based on [**Ledger Staking**](ledger::staking).
//!

use crate::abci::server::{callback::TENDERMINT_BLOCK_HEIGHT, forward_txn_with_mode};
use abci::{Evidence, Header, LastCommitInfo, PubKey, ValidatorUpdate};
use lazy_static::lazy_static;
use ledger::{
    data_model::{Transaction, TransferType, TxoRef, TxoSID, Utxo, ASSET_TYPE_FRA},
    staking::{
        ops::governance::{governance_penalty_tendermint_auto, ByzantineKind},
        td_pubkey_to_td_addr_bytes, Staking,
    },
    store::{LedgerAccess, LedgerUpdate},
};
use rand_core::{CryptoRng, RngCore};
use ruc::*;
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::Ordering,
};
use txn_builder::TransferOperationBuilder;
use zei::xfr::asset_record::{open_blind_asset_record, AssetRecordType};
use zei::xfr::{
    sig::{XfrKeyPair, XfrPublicKey},
    structs::{AssetRecordTemplate, XfrAmount, XfrAssetType},
};

mod whoami;

// The top 50 candidate validators
// will become official validators.
const VALIDATOR_LIMIT: usize = 50;

lazy_static! {
    /// Tendermint node address, sha256(pubkey)[:20]
    pub static ref TD_NODE_SELF_ADDR: Vec<u8> = pnk!(whoami::get_self_addr());
}

/// Get the effective validators at current block height.
pub fn get_validators(staking: &Staking) -> Result<Vec<ValidatorUpdate>> {
    let mut vs = staking
        .validator_get_current()
        .c(d!())?
        .body
        .values()
        .map(|v| (&v.td_pubkey, v.td_power))
        .collect::<Vec<_>>();

    // Ensure the minimal amount of BFT-like algorithm
    if 3 > vs.len() {
        return Err(eg!("invalid settings"));
    }

    // reverse sort
    vs.sort_by(|a, b| b.1.cmp(&a.1));

    // set the power of every extra validators to zero,
    // then tendermint can remove them from consensus logic.
    vs.iter_mut().skip(VALIDATOR_LIMIT).for_each(|(_, power)| {
        *power = 0;
    });

    Ok(vs
        .iter()
        .map(|(pubkey, power)| {
            let mut vu = ValidatorUpdate::new();
            let mut pk = PubKey::new();
            pk.set_field_type("ed25519".to_owned());
            pk.set_data(pubkey.to_vec());
            // this conversion is safe in the context of tendermint
            vu.set_power(*power as i64);
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
            };

            ruc::info_omit!(system_governance(staking, &bz));
        });

    if let Some(lci) = last_commit_info {
        let offline_list = lci
            .votes
            .iter()
            .filter(|v| !v.signed_last_block)
            .flat_map(|info| info.validator.as_ref().map(|v| &v.address))
            .collect::<HashSet<_>>();
        if let Ok(olpl) = ruc::info!(gen_offline_punish_list(staking, &offline_list)) {
            olpl.into_iter().for_each(|v| {
                let bz = ByzantineInfo {
                    addr: &hex::encode(v),
                    kind: "OFF_LINE",
                };
                ruc::info_omit!(system_governance(staking, &bz));
            });
        }
    }
}

// Get the actual total power of last block.
fn get_last_vote_power(last_commit_info: &LastCommitInfo) -> u64 {
    last_commit_info
        .votes
        .iter()
        .filter(|v| v.signed_last_block)
        .flat_map(|info| info.validator.as_ref().map(|v| v.power as u64))
        .sum()
}

// Set delegation rewards and proposer rewards
fn set_rewards(
    staking: &mut Staking,
    proposer: &[u8],
    last_vote_power: Option<u64>,
) -> Result<()> {
    staking
        .set_last_block_rewards(&hex::encode_upper(proposer), last_vote_power)
        .c(d!())
}

struct ByzantineInfo<'a> {
    addr: &'a str,
    // - "UNKNOWN"
    // - "DUPLICATE_VOTE"
    // - "LIGHT_CLIENT_ATTACK"
    kind: &'a str,
}

// Auto governance.
fn system_governance(staking: &mut Staking, bz: &ByzantineInfo) -> Result<()> {
    let kind = match bz.kind {
        "DUPLICATE_VOTE" => ByzantineKind::DuplicateVote,
        "LIGHT_CLIENT_ATTACK" => ByzantineKind::LightClientAttack,
        "OFF_LINE" => ByzantineKind::OffLine,
        "UNKNOWN" => ByzantineKind::Unknown,
        _ => return Err(eg!()),
    };
    governance_penalty_tendermint_auto(staking, bz.addr, &kind).c(d!())
}

// Pay for unbond 'Delegations' and 'FraDistributions'.
fn system_pay(la: &impl LedgerAccess, proposer: &[u8], fwder: &str) -> Result<()> {
    if *TD_NODE_SELF_ADDR != proposer {
        return Ok(());
    }

    let staking = la.get_staking();

    // at most 256 items to pay per block
    let mut paylist = staking
        .delegation_get_global_rewards()
        .into_iter()
        .chain(
            staking
                .fra_distribution_get_plan()
                .iter()
                .map(|(k, v)| (*k, *v)),
        )
        .take(256)
        .collect::<Vec<_>>();

    let mut principal_paylist = staking
        .delegation_get_global_principal()
        .into_iter()
        .take(256)
        .collect::<Vec<_>>();

    // sort by amount
    paylist.sort_by_key(|i| i.1);
    principal_paylist.sort_by_key(|i| i.1);

    macro_rules! pay {
        ($is_principal: expr, $utxos: expr, $paylist: expr) => {
            gen_transaction(la, $utxos, $paylist, $is_principal)
                .c(d!())
                .and_then(|tx| forward_txn_with_mode(fwder, tx, true).c(d!()))
        };
    }

    let coinbase_utxo_sids = staking.coinbase_txos().into_iter().collect::<Vec<_>>();
    let coinbase_principal_utxo_sids = staking
        .coinbase_principal_txos()
        .into_iter()
        .collect::<Vec<_>>();

    if !paylist.is_empty() {
        ruc::info_omit!(pay!(false, coinbase_utxo_sids, paylist));
    }

    if !principal_paylist.is_empty() {
        ruc::info_omit!(pay!(true, coinbase_principal_utxo_sids, principal_paylist));
    }

    Ok(())
}

// Generate all tx in batch mode.
fn gen_transaction(
    la: &impl LedgerAccess,
    utxo_sids: Vec<TxoSID>,
    paylist: Vec<(XfrPublicKey, u64)>,
    is_principal: bool,
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

    do_gen_transaction(
        inputs,
        outputs,
        seq_id,
        alt!(
            is_principal,
            staking.coinbase_principal_keypair(),
            staking.coinbase_keypair()
        ),
    )
    .c(d!())
}

// **NOTE:**
// transfer from CoinBase need not to pay FEE
fn do_gen_transaction(
    inputs: Vec<(TxoSID, Utxo, u64)>,
    dests: HashMap<XfrPublicKey, u64>,
    seq_id: u64,
    keypair: &XfrKeyPair,
) -> Result<Transaction> {
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

fn gen_offline_punish_list(
    staking: &Staking,
    voted_list: &HashSet<&Vec<u8>>,
) -> Result<Vec<Vec<u8>>> {
    let last_height = TENDERMINT_BLOCK_HEIGHT
        .load(Ordering::Relaxed)
        .saturating_sub(1);
    let mut vs = staking
        .validator_get_effective_at_height(last_height as u64)
        .c(d!())?
        .body
        .values()
        .map(|v| (td_pubkey_to_td_addr_bytes(&v.td_pubkey), v.td_power))
        .collect::<Vec<_>>();
    vs.sort_by(|a, b| b.1.cmp(&a.1));
    vs.iter_mut().skip(VALIDATOR_LIMIT).for_each(|(_, power)| {
        *power = 0;
    });

    Ok(vs
        .into_iter()
        .filter(|v| 0 < v.1 && !voted_list.contains(&v.0))
        .map(|(id, _)| id)
        .collect())
}

#[cfg(test)]
#[cfg(feature = "abci_mock")]
pub mod abci_mock_test;
