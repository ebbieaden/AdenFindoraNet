//!
//! # Staking
//!
//! Business logic based on [**Ledger Staking**](ledger::staking).
//!

use crate::abci::server::{forward_txn_with_mode, TD_NODE_SELF_ADDR};
use abci::{Evidence, Header, LastCommitInfo, PubKey, ValidatorUpdate};
use ledger::{
    data_model::{Transaction, TransferType, TxoRef, TxoSID, Utxo, ASSET_TYPE_FRA},
    staking::Staking,
    store::{LedgerAccess, LedgerUpdate},
};
use rand_core::{CryptoRng, RngCore};
use ruc::*;
use txn_builder::TransferOperationBuilder;
use zei::xfr::asset_record::{open_blind_asset_record, AssetRecordType};
use zei::xfr::{
    sig::XfrPublicKey,
    structs::{AssetRecordTemplate, XfrAmount, XfrAssetType},
};

#[cfg(test)]
#[cfg(feature = "abci_mock")]
mod abci_mock_test;

type SignedPower = i64;

// The top 50 candidate validators
// will become official validators.
const VALIDATOR_LIMIT: usize = 50;

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

    Ok(vs[..VALIDATOR_LIMIT]
        .iter()
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
                addr: &v.address,
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
        .set_last_block_rewards(proposer, last_vote_power)
        .c(d!())
}

#[allow(dead_code)]
struct ByzantineInfo<'a> {
    addr: &'a [u8],
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
    staking.governance_penalty(bz.addr, i64::MAX).c(d!())
}

// Pay for frozen 'Delegations' and 'FraDistributions'.
fn system_pay(la: &impl LedgerAccess, proposer: &[u8], fwder: &str) -> Result<()> {
    if *TD_NODE_SELF_ADDR != proposer {
        return Ok(());
    }

    let staking = la.get_staking();

    let mut paylist = staking
        .delegation_get_rewards()
        .into_iter()
        .chain(staking.fra_distribution_get_plan().into_iter())
        .collect::<Vec<_>>();

    // sort by amount
    paylist.sort_by_key(|i| i.1);

    let coinbase_utxo_sids = staking.coinbase_txos().into_iter().collect::<Vec<_>>();

    gen_transaction_list(la, coinbase_utxo_sids, paylist)
        .into_iter()
        .for_each(|tx| {
            // send tx in 'async' mode
            ruc::info_omit!(forward_txn_with_mode(fwder, tx, true));
        });

    Ok(())
}

// Generate all tx in batch mode.
fn gen_transaction_list(
    la: &impl LedgerAccess,
    utxo_sids: Vec<TxoSID>,
    paylist: Vec<(XfrPublicKey, u64)>,
) -> Vec<Transaction> {
    let staking = la.get_staking();
    let seq_id = la.get_state_commitment().1;

    let mut res = vec![];
    let mut sids = utxo_sids.into_iter();

    'fin: for (addr, am) in paylist.into_iter() {
        let mut am2 = am;
        let mut inputs = vec![];
        while am2 > 0 {
            if let Some(sid) = sids.next() {
                if let Some(auth_utxo) = la.get_utxo(sid) {
                    let utxo = auth_utxo.utxo;
                    if let XfrAssetType::NonConfidential(ty) = utxo.0.record.asset_type {
                        if ASSET_TYPE_FRA == ty {
                            if let XfrAmount::NonConfidential(i_am) =
                                utxo.0.record.amount
                            {
                                am2 = am2.saturating_sub(i_am);
                                inputs.push((sid, utxo, i_am));
                            }
                        }
                    }
                }
            } else {
                // insufficient balance in coinbase
                break 'fin;
            }
        }
        if let Ok(tx) = ruc::info!(gen_transaction(staking, inputs, addr, am, seq_id)) {
            res.push(tx);
        }
    }

    res
}

// **NOTE:**
// transfer from CoinBase need not to pay FEE
fn gen_transaction(
    staking: &Staking,
    inputs: Vec<(TxoSID, Utxo, u64)>,
    dest: XfrPublicKey,
    am: u64,
    seq_id: u64,
) -> Result<Transaction> {
    let keypair = staking.coinbase_keypair();

    let output_template = AssetRecordTemplate::with_no_asset_tracing(
        am,
        ASSET_TYPE_FRA,
        AssetRecordType::NonConfidentialAmount_NonConfidentialAssetType,
        dest,
    );

    let mut op = TransferOperationBuilder::new();

    for (sid, utxo, i_am) in inputs.into_iter() {
        op.add_input(
            TxoRef::Absolute(sid),
            open_blind_asset_record(&utxo.0.record, &None, keypair).unwrap(),
            None,
            None,
            i_am,
        )
        .c(d!())?;
    }

    let op = op
        .add_output(&output_template, None, None, None)
        .c(d!())?
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
