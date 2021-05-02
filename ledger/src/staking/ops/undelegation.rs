//!
//! # Un-Delegation Operation
//!
//! Data representation required when users propose a un-delegation.
//!

use crate::{
    data_model::{Operation, Transaction},
    staking::Staking,
};
use ruc::*;
use serde::{Deserialize, Serialize};
use zei::xfr::sig::{XfrPublicKey, XfrSignature};

/// Used as the inner object of a `UnDelegation Operation`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnDelegationOps {
    body: Data,
    pub(crate) pubkey: XfrPublicKey,
    signature: XfrSignature,
}

impl UnDelegationOps {
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
            .and_then(|_| staking.undelegate(&self.pubkey).c(d!()))
    }

    /// Verify signature.
    pub fn verify(&self) -> Result<()> {
        self.body
            .to_bytes()
            .c(d!())
            .and_then(|d| self.pubkey.verify(&d, &self.signature).c(d!()))
    }

    #[inline(always)]
    fn check_context(tx: &Transaction) -> Result<()> {
        check_delegation_context(tx).c(d!())
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_related_pubkeys(&self) -> Vec<XfrPublicKey> {
        vec![self.pubkey]
    }
}

// The body of a delegation operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Data {
    uuid: u64,
}

impl Data {
    #[inline(always)]
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).c(d!())
    }
}

// Check tx and return the amount of delegation.
// - total amount of operations is 2
// - the first one is a `TransferAsset` to pay fee
// - the second one is a `UnDelegation`
fn check_delegation_context(tx: &Transaction) -> Result<()> {
    if 2 != tx.body.operations.len() {
        return Err(eg!("incorrect number of operations"));
    }

    // 1. the first operation must be a FEE operation
    check_delegation_context_fee(tx).c(d!("invalid fee operation"))?;

    // 2. the second operation must be a `UnDelegation` operation
    if matches!(tx.body.operations[1], Operation::UnDelegation(_)) {
        Ok(())
    } else {
        Err(eg!())
    }
}

#[inline(always)]
fn check_delegation_context_fee(tx: &Transaction) -> Result<()> {
    super::delegation::check_delegation_context_fee(tx).c(d!())
}
