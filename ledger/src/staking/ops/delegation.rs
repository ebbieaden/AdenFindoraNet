//!
//! # Delegation Operation
//!
//! Data representation required when users propose a delegation.
//!

use crate::{
    data_model::{
        NoReplayToken, Operation, Transaction, ASSET_TYPE_FRA, BLACK_HOLE_PUBKEY,
    },
    staking::{Amount, Staking, TendermintAddr, COINBASE_PRINCIPAL_PK},
};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, convert::TryFrom};
use zei::xfr::{
    sig::{XfrKeyPair, XfrPublicKey, XfrSignature},
    structs::{XfrAmount, XfrAssetType},
};

/// Used as the inner object of a `Delegation Operation`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelegationOps {
    pub(crate) body: Data,
    pub(crate) pubkey: XfrPublicKey,
    signature: XfrSignature,
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
                    .delegate(self.pubkey, &self.body.validator, am, staking.cur_height)
                    .c(d!())
            })
    }

    /// Verify signature.
    #[inline(always)]
    pub fn verify(&self) -> Result<()> {
        self.pubkey
            .verify(&self.body.to_bytes(), &self.signature)
            .c(d!())
    }

    #[inline(always)]
    fn check_context(tx: &Transaction) -> Result<Amount> {
        check_delegation_context(tx).c(d!())
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_related_pubkeys(&self) -> Vec<XfrPublicKey> {
        vec![self.pubkey]
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new(
        keypair: &XfrKeyPair,
        validator: TendermintAddr,
        nonce: NoReplayToken,
    ) -> Self {
        let body = Data::new(validator, nonce);
        let signature = keypair.sign(&body.to_bytes());
        DelegationOps {
            body,
            pubkey: keypair.get_pk(),
            signature,
        }
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn set_nonce(&mut self, nonce: NoReplayToken) {
        self.body.set_nonce(nonce);
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_nonce(&self) -> NoReplayToken {
        self.body.get_nonce()
    }
}

/// The body of a delegation operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// the target validator to delegated to
    pub validator: TendermintAddr,
    nonce: NoReplayToken,
}

impl Data {
    #[inline(always)]
    fn new(v: TendermintAddr, nonce: NoReplayToken) -> Self {
        Data {
            validator: v,
            nonce,
        }
    }

    #[inline(always)]
    fn to_bytes(&self) -> Vec<u8> {
        pnk!(bincode::serialize(self))
    }

    #[inline(always)]
    fn set_nonce(&mut self, nonce: NoReplayToken) {
        self.nonce = nonce;
    }

    #[inline(always)]
    fn get_nonce(&self) -> NoReplayToken {
        self.nonce
    }
}

// Check tx and return the amount of delegation.
// - total amount of operations is 3
// - one of them is a `TransferAsset` to pay fee
// - one of them  is a `Delegation`
// - one of them  is a `TransferAsset` to pay to CoinBasePrincipal
//     - all inputs must be owned by a same address
//     - number of its outputs must be 1,
//     - and this output must be `NonConfidential`
//     - and this output will be used as the amount of delegation
fn check_delegation_context(tx: &Transaction) -> Result<Amount> {
    if 3 != tx.body.operations.len() {
        return Err(eg!("incorrect number of operations"));
    }

    // 1. check FEE operation
    check_delegation_context_fee(tx, 3).c(d!("invalid fee operation"))?;

    // 2. check `Delegation` operation
    let owner = (0..3)
        .filter_map(|i| {
            if let Operation::Delegation(ref x) = tx.body.operations[i] {
                Some(x.pubkey)
            } else {
                None
            }
        })
        .next()
        .ok_or(eg!("delegation ops not found"))?;

    // 3. check non-confidential self-`TransferAsset`
    check_delegation_context_principal(tx, owner, 3)
        .c(d!("delegation amount is not paid correctly"))
}

// if balance output exists,
// target addr must be same as the inputs
macro_rules! check_balance {
    ($op: expr, $action: block) => {
        if let Some(o) = $op.body.outputs.get(1) {
            if 1 == $op
                .body
                .transfer
                .inputs
                .iter()
                .map(|i| i.public_key)
                .collect::<HashSet<_>>()
                .len()
                && o.record.public_key == $op.body.transfer.inputs[0].public_key
            {
                $action
            }
        } else {
            $action
        }
    };
}

pub(crate) fn check_delegation_context_fee(
    tx: &Transaction,
    total: usize,
) -> Result<()> {
    let valid = (0..total).any(|i| {
        if let Some(Operation::TransferAsset(ref x)) = tx.body.operations.get(i) {
            // more than 2 outputs is not allowed
            if 2 < x.body.outputs.len() {
                return false;
            }

            let o = &x.body.outputs[0];
            if let XfrAssetType::NonConfidential(ty) = o.record.asset_type {
                if ty == ASSET_TYPE_FRA && *BLACK_HOLE_PUBKEY == o.record.public_key {
                    if let XfrAmount::NonConfidential(_) = o.record.amount {
                        check_balance!(x, {
                            return true;
                        });
                    }
                }
            }
        }
        false
    });

    alt!(valid, Ok(()), Err(eg!()))
}

fn check_delegation_context_principal(
    tx: &Transaction,
    owner: XfrPublicKey,
    total: usize,
) -> Result<Amount> {
    let mut am = None;
    let target_pk = *COINBASE_PRINCIPAL_PK;

    for i in 0..total {
        if let Some(Operation::TransferAsset(ref x)) = tx.body.operations.get(i) {
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
                continue;
            }

            // ensure the owner of all inputs is same as the delegater.
            if owner != x.body.transfer.inputs[0].public_key {
                continue;
            }

            // more than 2 outputs is not allowed
            if 2 < x.body.outputs.len() {
                continue;
            }

            let o = &x.body.outputs[0];
            if let XfrAssetType::NonConfidential(ty) = o.record.asset_type {
                if ty == ASSET_TYPE_FRA && target_pk == o.record.public_key {
                    if let XfrAmount::NonConfidential(i_am) = o.record.amount {
                        check_balance!(x, {
                            am = Amount::try_from(i_am).ok();
                            break;
                        });
                    }
                }
            }
        }
    }

    am.ok_or(eg!())
}
