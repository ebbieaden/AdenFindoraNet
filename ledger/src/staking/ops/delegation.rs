//!
//! # Delegation Operation
//!
//! Data representation required when users propose a delegation.
//!

use crate::{
    data_model::{Operation, Transaction, ASSET_TYPE_FRA, BLACK_HOLE_PUBKEY},
    staking::{Amount, Staking},
};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, convert::TryFrom};
use zei::xfr::{
    sig::{XfrPublicKey, XfrSignature},
    structs::{XfrAmount, XfrAssetType},
};

/// Used as the inner object of a `Delegation Operation`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelegationOps {
    pub(crate) body: Data,
    pub(crate) pubkey: XfrPublicKey,
    pub(crate) signature: XfrSignature,
}

impl DelegationOps {
    /// Check the validity of an operation by running it in a staking simulator.
    #[inline(always)]
    pub fn check_run(
        &self,
        staking_simulator: &mut Staking,
        tx: &Transaction,
    ) -> Result<()> {
        self.apply(staking_simulator, tx).c(d!())
    }

    /// Apply new delegation to the target `Staking` instance.
    pub fn apply(&self, staking: &mut Staking, tx: &Transaction) -> Result<()> {
        self.verify()
            .c(d!())
            .and_then(|_| Self::check_context(tx).c(d!()))
            .and_then(|am| {
                staking
                    .delegate(
                        self.pubkey,
                        self.body.validator,
                        am,
                        staking.cur_height,
                        staking.cur_height.saturating_add(self.body.block_span),
                    )
                    .c(d!())
            })
    }

    /// Verify signature.
    pub fn verify(&self) -> Result<()> {
        self.body
            .to_bytes()
            .c(d!())
            .and_then(|d| self.pubkey.verify(&d, &self.signature).c(d!()))
    }

    #[inline(always)]
    fn check_context(tx: &Transaction) -> Result<Amount> {
        check_delegation_context(tx).c(d!())
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_related_pubkeys(&self) -> Vec<XfrPublicKey> {
        vec![self.pubkey, self.body.validator]
    }
}

type BlockAmount = u64;

/// The body of a delegation operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// the target validator to delegated to
    pub validator: XfrPublicKey,
    /// how many heights should this delegation be locked
    ///
    /// **NOTE:** before users can actually get the rewards,
    /// they need to wait for an extra `FROZEN_BLOCK_CNT` period
    pub block_span: BlockAmount,
}

impl Data {
    #[inline(always)]
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).c(d!())
    }
}

// Check tx and return the amount of delegation.
// - total amount of operations is 3
// - the first one is a `TransferAsset` to pay fee
// - the second one is a `Deletation`
// - the third one is a `TransferAsset` to pay to self
//     - all inputs must be owned by a same address
//     - number of its outputs must be 1,
//     - and this output must be `NonConfidential`
//     - and this output will be used as the amount of delegation
fn check_delegation_context(tx: &Transaction) -> Result<Amount> {
    if 3 != tx.body.operations.len() {
        return Err(eg!("incorrect number of operations"));
    }

    // 1. the first operation must be a FEE operation
    check_delegation_context_fee(tx).c(d!("invalid fee operation"))?;

    // 2. the second operation must be a `Delegation` operation
    let owner = if let Operation::Delegation(ref x) = tx.body.operations[1] {
        x.pubkey
    } else {
        return Err(eg!("delegation not found"));
    };

    // 3. the third operation must be a non-confidential `TransferAsset` to self
    check_delegation_context_self_transfer(tx, owner)
        .c(d!("delegation amount is not paid correctly"))
}

pub(crate) fn check_delegation_context_fee(tx: &Transaction) -> Result<()> {
    if let Operation::TransferAsset(ref x) = tx.body.operations.get(0).ok_or(eg!())? {
        if 1 != x.body.outputs.len() {
            return Err(eg!("multi outputs is not allowed"));
        }

        let o = &x.body.outputs[0];
        if let XfrAssetType::NonConfidential(ty) = o.record.asset_type {
            if ty == ASSET_TYPE_FRA && *BLACK_HOLE_PUBKEY == o.record.public_key {
                if let XfrAmount::NonConfidential(_) = o.record.amount {
                    return Ok(());
                }
            }
        }
    }

    Err(eg!())
}

fn check_delegation_context_self_transfer(
    tx: &Transaction,
    owner: XfrPublicKey,
) -> Result<Amount> {
    if let Operation::TransferAsset(ref x) = tx.body.operations.get(2).ok_or(eg!())? {
        // ensure all inputs are owned by a same address.
        if 1 != x
            .body
            .transfer
            .inputs
            .iter()
            .map(|i| i.public_key)
            .collect::<HashSet<_>>()
            .len()
        {
            return Err(eg!("multi owners is not allowed"));
        }

        // ensure the owner of all inputs is same as the delegater.
        if owner != x.body.transfer.inputs[0].public_key {
            return Err(eg!("pubkey not match"));
        }

        if 1 != x.body.outputs.len() {
            return Err(eg!("multi outputs is not allowed"));
        }

        let o = &x.body.outputs[0];
        if let XfrAssetType::NonConfidential(ty) = o.record.asset_type {
            if ty == ASSET_TYPE_FRA && owner == o.record.public_key {
                if let XfrAmount::NonConfidential(am) = o.record.amount {
                    return Amount::try_from(am).c(d!()); // all is well
                }
            }
        }
    }

    Err(eg!())
}

/// Transfer assets from delegated address is not allowed,
/// except the unique `TransferAsset` operation in the delegation transaction.
///
/// Detect whether there are some delegated addresses in `tx`;
/// If detected, return true, otherwise return false.
///
/// Rules:
///     1. this transaction is not a 'delegation'
///     2. this transaction contains delegated addresses in its `inputs`
pub fn found_delegated_addresses(staking: &Staking, tx: &Transaction) -> bool {
    check_delegation_context(tx).is_err()
        && tx.body.operations.iter().any(|o| {
            if let Operation::TransferAsset(ref x) = o {
                return x
                    .body
                    .transfer
                    .inputs
                    .iter()
                    .any(|i| staking.di.addr_map.contains_key(&i.public_key));
            }
            false
        })
}
