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
        td_addr_to_string, Staking, COINBASE_PAYMENT_BLOCK_ITV, COINBASE_PK,
        COINBASE_PRINCIPAL_PK, VALIDATOR_UPDATE_BLOCK_ITV,
    },
    store::{LedgerAccess, LedgerUpdate},
};
use rand_core::{CryptoRng, RngCore};
use ruc::*;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::atomic::Ordering,
};
use txn_builder::TransferOperationBuilder;
use zei::xfr::asset_record::{open_blind_asset_record, AssetRecordType};
use zei::xfr::{
    sig::{XfrKeyPair, XfrPublicKey},
    structs::{AssetRecordTemplate, OwnerMemo, XfrAmount, XfrAssetType},
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
///
/// > #### Tendermint Rules
/// >
/// > Validator updates returned by block H impact blocks H+1, H+2, and H+3,
/// > but only effects changes on the validator set of H+2:
/// > - H+1: NextValidatorsHash
/// > - H+2: ValidatorsHash (and thus the validator set)
/// > - H+3: LastCommitInfo (ie. the last validator set)
/// > - Consensus params returned for block H apply for block H+1
/// >
/// > The pub_key currently supports only one type:
/// > - type = "ed25519"
/// >
/// > The power is the new voting power for the validator, with the following rules:
/// > - power must be non-negative
/// >   - if power is 0, the validator must already exist, and will be removed from the validator set
/// >   - if power is non-0:
/// >     - if the validator does not already exist, it will be added to the validator set with the given power
/// >     - if the validator does already exist, its power will be adjusted to the given power
/// > - the total power of the new validator set must not exceed MaxTotalVotingPower
pub fn get_validators(
    staking: &Staking,
    last_commit_info: Option<&LastCommitInfo>,
) -> Result<Option<Vec<ValidatorUpdate>>> {
    // Update the validator list every 4 blocks to ensure that
    // the validator list obtained from `LastCommitInfo` is exactly
    // the same as the current block.
    // So we can use it to filter out non-existing entries.
    if 0 != TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed) % VALIDATOR_UPDATE_BLOCK_ITV
    {
        return Ok(None);
    }

    // Get existing entries in the last block.
    let last_entries = if let Some(lci) = last_commit_info {
        lci.votes
            .as_slice()
            .iter()
            .flat_map(|v| v.validator.as_ref().map(|v| (&v.address, v.power)))
            .collect::<HashMap<_, _>>()
    } else {
        map! {}
    };

    // The logic of the context guarantees:
    // - current entries == last entries
    let cur_entries = last_entries;

    let mut vs = staking
        .validator_get_current()
        .c(d!())?
        .body
        .values()
        .filter(|v| {
            if let Some(power) = cur_entries.get(&v.td_addr) {
                // - new power > 0: change existing entries
                // - new power = 0: remove existing entries
                // - the power returned by `LastCommitInfo` is impossible
                // to be zero in the context of tendermint
                *power as u64 != v.td_power
            } else {
                // add new validator
                //
                // try to remove non-existing entries is not allowed
                0 < v.td_power
            }
        })
        .map(|v| (&v.td_pubkey, v.td_power))
        .collect::<Vec<_>>();

    if vs.is_empty() {
        return Ok(None);
    }

    // reverse sort
    vs.sort_by(|a, b| b.1.cmp(&a.1));

    // set the power of every extra validators to zero,
    // then tendermint can remove them from consensus logic.
    vs.iter_mut().skip(VALIDATOR_LIMIT).for_each(|(_, power)| {
        *power = 0;
    });

    Ok(Some(
        vs.iter()
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
            .collect(),
    ))
}

// Call this function in `EndBlock`,
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
    // trigger system staking process
    la.get_staking_mut().delegation_process();
    la.get_staking_mut().validator_apply_current();

    ruc::info_omit!(set_rewards(
        la.get_staking_mut(),
        &header.proposer_address,
        last_commit_info.map(|lci| get_last_vote_percent(lci))
    ));

    // tendermint primary governances
    evs.iter()
        .filter(|ev| ev.validator.is_some())
        .for_each(|ev| {
            let v = ev.validator.as_ref().unwrap();
            let bz = ByzantineInfo {
                addr: &td_addr_to_string(&v.address),
                kind: ev.field_type.as_str(),
            };

            ruc::info_omit!(system_governance(la.get_staking_mut(), &bz));
        });

    // application custom governances
    if let Some(lci) = last_commit_info {
        let offline_list = lci
            .votes
            .iter()
            .filter(|v| !v.signed_last_block)
            .flat_map(|info| info.validator.as_ref().map(|v| &v.address))
            .collect::<HashSet<_>>();

        let staking = la.get_staking_mut();

        // mark if a validator is online at last block
        if let Ok(vd) = ruc::info!(staking.validator_get_current_mut()) {
            vd.body.values_mut().for_each(|v| {
                if offline_list.contains(&v.td_addr) {
                    v.signed_last_block = false;
                } else {
                    v.signed_last_block = true;
                }
            });
        }

        if !offline_list.is_empty() {
            if let Ok(olpl) = ruc::info!(gen_offline_punish_list(staking, &offline_list))
            {
                olpl.into_iter().for_each(|v| {
                    let bz = ByzantineInfo {
                        addr: &td_addr_to_string(&v),
                        kind: "OFF_LINE",
                    };
                    ruc::info_omit!(system_governance(la.get_staking_mut(), &bz));
                });
            }
        }
    }

    if 0 == TENDERMINT_BLOCK_HEIGHT.load(Ordering::Relaxed) % COINBASE_PAYMENT_BLOCK_ITV
    {
        // In a real consensus cluster, there is no guarantee that
        // transactions sent by CoinBase will be confirmed in the next block due to asynchronous delays.
        //
        // If this happens, CoinBase will send repeated payment transactions.
        //
        // Although these repeated transactions will eventually fail,
        // they will give users a bad experience and increase the load of p2p cluster.
        //
        // Therefore, paying every 4 blocks seems to be a good compromise.
        ruc::info_omit!(system_pay(la, &header.proposer_address, fwder));
    }

    // clean validators with zero power
    clean_outdated_validators(la.get_staking_mut());
}

// Get the actual voted power of last block.
fn get_last_vote_percent(last_commit_info: &LastCommitInfo) -> [u64; 2] {
    last_commit_info
        .votes
        .iter()
        .flat_map(|info| {
            info.validator
                .as_ref()
                .map(|v| [alt!(info.signed_last_block, v.power, 0), v.power])
        })
        .fold([0, 0], |mut acc, i| {
            // this `AddAsign` is safe in the context of tendermint
            acc[0] += i[0] as u64;
            acc[1] += i[1] as u64;
            acc
        })
}

// Set delegation rewards and proposer rewards
fn set_rewards(
    staking: &mut Staking,
    proposer: &[u8],
    last_vote_percent: Option<[u64; 2]>,
) -> Result<()> {
    staking
        .set_last_block_rewards(&td_addr_to_string(proposer), last_vote_percent)
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
        .take(512)
        .collect::<Vec<_>>();

    let mut principal_paylist = staking
        .delegation_get_global_principal()
        .into_iter()
        .take(512)
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

    let coinbase_utxos = la.get_owned_utxos(&COINBASE_PK);
    let coinbase_principal_utxos = la.get_owned_utxos(&COINBASE_PRINCIPAL_PK);

    if !paylist.is_empty() {
        ruc::info_omit!(pay!(false, coinbase_utxos, paylist));
    }

    if !principal_paylist.is_empty() {
        ruc::info_omit!(pay!(true, coinbase_principal_utxos, principal_paylist));
    }

    Ok(())
}

// Generate all tx in batch mode.
fn gen_transaction(
    la: &impl LedgerAccess,
    utxos: BTreeMap<TxoSID, (Utxo, Option<OwnerMemo>)>,
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

        let mut utxos = utxos.iter();
        inputs.clear();

        while total_amount > 0 {
            if let Some((sid, (utxo, owner_memo))) = utxos.next() {
                if let XfrAssetType::NonConfidential(ty) = utxo.0.record.asset_type {
                    if ASSET_TYPE_FRA == ty {
                        if let XfrAmount::NonConfidential(am) = utxo.0.record.amount {
                            inputs.push((
                                *sid,
                                utxo,
                                owner_memo,
                                alt!(total_amount > am, am, total_amount),
                            ));
                            total_amount = total_amount.saturating_sub(am);
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
    inputs: Vec<(TxoSID, &Utxo, &Option<OwnerMemo>, u64)>,
    dests: HashMap<XfrPublicKey, u64>,
    seq_id: u64,
    keypair: &XfrKeyPair,
) -> Result<Transaction> {
    let mut op = TransferOperationBuilder::new();

    for (sid, utxo, owner_memo, n) in inputs.into_iter() {
        op.add_input(
            TxoRef::Absolute(sid),
            open_blind_asset_record(&utxo.0.record, owner_memo, keypair).unwrap(),
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
    staking: &mut Staking,
    offline_list: &HashSet<&Vec<u8>>,
) -> Result<Vec<Vec<u8>>> {
    let last_height = TENDERMINT_BLOCK_HEIGHT
        .load(Ordering::Relaxed)
        .saturating_sub(1);
    let validators = staking
        .validator_get_effective_at_height_mut(last_height as u64)
        .c(d!())?;

    let mut vs = validators
        .body
        .values()
        .map(|v| (&v.td_addr, v.td_power))
        .collect::<Vec<_>>();
    vs.sort_by(|a, b| b.1.cmp(&a.1));
    vs.iter_mut().skip(VALIDATOR_LIMIT).for_each(|(_, power)| {
        *power = 0;
    });

    Ok(vs
        .into_iter()
        .filter(|v| 0 < v.1 && offline_list.contains(&v.0))
        .map(|(id, _)| id.clone())
        .collect())
}

// call this func after each round of ValidatorUpdate
fn clean_outdated_validators(staking: &mut Staking) {
    staking.validator_clean_invalid_items();
}

#[cfg(feature = "abci_mock")]
pub mod abci_mock_test;
