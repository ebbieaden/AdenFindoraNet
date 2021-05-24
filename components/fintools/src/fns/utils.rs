use super::get_serv_addr;
use ledger::data_model::{
    DelegationInfo, Operation, StateCommitmentData, Transaction, TransferType, TxoRef,
    TxoSID, Utxo, ASSET_TYPE_FRA, BLACK_HOLE_PUBKEY, TX_FEE_MIN,
};
use ruc::*;
use std::collections::HashMap;
use txn_builder::{BuildsTransactions, TransactionBuilder, TransferOperationBuilder};
use utils::{HashOf, SignatureOf};
use zei::xfr::{
    asset_record::{open_blind_asset_record, AssetRecordType},
    sig::{XfrKeyPair, XfrPublicKey},
    structs::{AssetRecordTemplate, OwnerMemo, XfrAmount},
};

///////////////////////////////////////
// Part 1: utils for transfer assets //
///////////////////////////////////////

#[inline(always)]
#[allow(missing_docs)]
pub fn new_tx_builder() -> Result<TransactionBuilder> {
    get_seq_id().c(d!()).map(TransactionBuilder::from_seq_id)
}

#[inline(always)]
#[allow(missing_docs)]
pub fn send_tx(tx: &Transaction) -> Result<()> {
    let url = format!("{}:8669/submit_transaction", get_serv_addr().c(d!())?);
    attohttpc::post(&url)
        .header(attohttpc::header::CONTENT_TYPE, "application/json")
        .bytes(&serde_json::to_vec(tx).c(d!())?)
        .send()
        .c(d!("fail to send transaction"))
        .map(|_| ())
}

#[inline(always)]
#[allow(missing_docs)]
pub fn transfer(owner_kp: &XfrKeyPair, target_pk: &XfrPublicKey, am: u64) -> Result<()> {
    transfer_batch(owner_kp, vec![(target_pk, am)]).c(d!())
}

#[inline(always)]
#[allow(missing_docs)]
pub fn transfer_batch(
    owner_kp: &XfrKeyPair,
    target_list: Vec<(&XfrPublicKey, u64)>,
) -> Result<()> {
    let mut builder = new_tx_builder().c(d!())?;
    let op = gen_transfer_op(owner_kp, target_list).c(d!())?;
    builder.add_operation(op);
    send_tx(&builder.take_transaction()).c(d!())
}

/// @target_list: use `Vec` but `HashMap` ?
///     there might be multi entries to one address
#[inline(always)]
pub fn gen_transfer_op(
    owner_kp: &XfrKeyPair,
    target_list: Vec<(&XfrPublicKey, u64)>,
) -> Result<Operation> {
    gen_transfer_op_x(owner_kp, target_list, true).c(d!())
}

#[allow(missing_docs)]
pub fn gen_transfer_op_x(
    owner_kp: &XfrKeyPair,
    mut target_list: Vec<(&XfrPublicKey, u64)>,
    auto_fee: bool,
) -> Result<Operation> {
    if auto_fee {
        target_list.push((&*BLACK_HOLE_PUBKEY, TX_FEE_MIN));
    }

    let mut trans_builder = TransferOperationBuilder::new();

    let mut am = target_list.iter().map(|(_, am)| *am).sum();
    let mut i_am;
    let utxos = get_owned_utxos(owner_kp.get_pk_ref()).c(d!())?.into_iter();

    for (sid, (utxo, owner_memo)) in utxos {
        if let XfrAmount::NonConfidential(n) = utxo.0.record.amount {
            alt!(n < am, i_am = n, i_am = am);
            am = am.saturating_sub(n);
        } else {
            continue;
        }

        open_blind_asset_record(&utxo.0.record, &owner_memo, owner_kp)
            .c(d!())
            .and_then(|ob| {
                trans_builder
                    .add_input(TxoRef::Absolute(sid), ob, None, None, i_am)
                    .c(d!())
            })?;

        alt!(0 == am, break);
    }

    if 0 != am {
        return Err(eg!("insufficient balance"));
    }

    let outputs = target_list.into_iter().map(|(pk, n)| {
        AssetRecordTemplate::with_no_asset_tracing(
            n,
            ASSET_TYPE_FRA,
            AssetRecordType::NonConfidentialAmount_NonConfidentialAssetType,
            *pk,
        )
    });

    for output in outputs {
        trans_builder
            .add_output(&output, None, None, None)
            .c(d!())?;
    }

    trans_builder
        .balance()
        .c(d!())?
        .create(TransferType::Standard)
        .c(d!())?
        .sign(owner_kp)
        .c(d!())?
        .transaction()
        .c(d!())
}

/// for scenes that need to pay a standalone fee without other transfers
#[inline(always)]
#[allow(missing_docs)]
pub fn gen_fee_op(owner_kp: &XfrKeyPair) -> Result<Operation> {
    gen_transfer_op(owner_kp, vec![]).c(d!())
}

/////////////////////////////////////////
// Part 2: utils for query infomations //
/////////////////////////////////////////

#[inline(always)]
#[allow(missing_docs)]
pub fn get_balance(kp: &XfrKeyPair) -> Result<u64> {
    let balance = get_owned_utxos(kp.get_pk_ref())
        .c(d!())?
        .values()
        .map(|(utxo, owner_memo)| {
            open_blind_asset_record(&utxo.0.record, owner_memo, kp)
                .c(d!())
                .map(|obr| obr.amount)
        })
        .collect::<Result<Vec<_>>>()
        .c(d!())?
        .iter()
        .sum();

    Ok(balance)
}

fn get_owned_utxos(
    addr: &XfrPublicKey,
) -> Result<HashMap<TxoSID, (Utxo, Option<OwnerMemo>)>> {
    let url = format!(
        "{}:8668/owned_utxos/{}",
        get_serv_addr().c(d!())?,
        wallet::public_key_to_base64(addr)
    );

    attohttpc::get(&url)
        .send()
        .c(d!())?
        .error_for_status()
        .c(d!())?
        .bytes()
        .c(d!())
        .and_then(|b| {
            serde_json::from_slice::<HashMap<TxoSID, (Utxo, Option<OwnerMemo>)>>(&b)
                .c(d!())
        })
}

#[inline(always)]
fn get_seq_id() -> Result<u64> {
    type Resp = (
        HashOf<Option<StateCommitmentData>>,
        u64,
        SignatureOf<(HashOf<Option<StateCommitmentData>>, u64)>,
    );

    let url = format!("{}:8668/global_state", get_serv_addr().c(d!())?);

    attohttpc::get(&url)
        .send()
        .c(d!())?
        .error_for_status()
        .c(d!())?
        .bytes()
        .c(d!())
        .and_then(|b| serde_json::from_slice::<Resp>(&b).c(d!()))
        .map(|resp| resp.1)
}

#[inline(always)]
#[allow(missing_docs)]
pub fn get_owner_memo_batch(ids: &[TxoSID]) -> Result<Vec<Option<OwnerMemo>>> {
    let ids = ids
        .iter()
        .map(|id| id.0.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let url = format!(
        "{}:8667/get_owner_memo_batch/{}",
        get_serv_addr().c(d!())?,
        ids
    );

    attohttpc::get(&url)
        .send()
        .c(d!())?
        .error_for_status()
        .c(d!())?
        .bytes()
        .c(d!())
        .and_then(|b| serde_json::from_slice(&b).c(d!()))
}

/// basic delegation info, and staking info if `pk` is a validator
pub fn get_delegation_info(pk: &XfrPublicKey) -> Result<DelegationInfo> {
    let url = format!(
        "{}:8668/delegation_info/{}",
        get_serv_addr().c(d!())?,
        wallet::public_key_to_base64(pk)
    );

    attohttpc::get(&url)
        .send()
        .c(d!())?
        .error_for_status()
        .c(d!())?
        .bytes()
        .c(d!())
        .and_then(|b| serde_json::from_slice::<DelegationInfo>(&b).c(d!()))
}