//!
//! # FRA Distribution
//!
//! Used to support the distribution of the official token FRA.
//!
//! **NOTE**: always use the same multi-signature rules as `UpdateValidator`.
//!

use crate::{
    data_model::{NoReplayToken, Operation, Transaction},
    staking::{cosig::CoSigOp, Staking},
};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey};

/// Used as the inner object of a `FraDistribution Operation`.
pub type FraDistributionOps = CoSigOp<Data>;

impl FraDistributionOps {
    /// Check the validity of an operation by running it in a staking simulator.
    #[inline(always)]
    pub fn check_run(
        &self,
        staking_simulator: &mut Staking,
        tx: &Transaction,
    ) -> Result<()> {
        self.apply(staking_simulator, tx).c(d!())
    }

    /// Apply new settings to the target `Staking` instance.
    #[inline(always)]
    pub fn apply(&self, staking: &mut Staking, tx: &Transaction) -> Result<()> {
        self.verify(staking)
            .c(d!())
            .and_then(|_| Self::check_context(tx).c(d!()))
            .and_then(|_| {
                staking
                    .coinbase_config_fra_distribution(self.clone())
                    .c(d!())
            })
    }

    #[inline(always)]
    fn check_context(tx: &Transaction) -> Result<()> {
        check_fra_distribution_context(tx).c(d!())
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_related_pubkeys(&self) -> Vec<XfrPublicKey> {
        self.cosigs
            .keys()
            .chain(self.data.alloc_table.keys())
            .copied()
            .collect()
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new(
        kps: &[&XfrKeyPair],
        alloc_table: BTreeMap<XfrPublicKey, u64>,
        nonce: NoReplayToken,
    ) -> Result<Self> {
        let mut op = CoSigOp::create(Data::new(alloc_table), nonce);
        op.batch_sign(kps).c(d!()).map(|_| op)
    }
}

/// The body of a `FraDistribution Operation`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// How many FRAs to pay for each address.
    pub alloc_table: BTreeMap<XfrPublicKey, u64>,
}

impl Data {
    #[inline(always)]
    fn new(alloc_table: BTreeMap<XfrPublicKey, u64>) -> Self {
        Data { alloc_table }
    }
}

// Check tx and return the amount of delegation.
// - total amount of operations is 2
// - one of them is a `TransferAsset` to pay fee
// - one of them is a `FraDistribution`
fn check_fra_distribution_context(tx: &Transaction) -> Result<()> {
    if 2 != tx.body.operations.len() {
        return Err(eg!("incorrect number of operations"));
    }

    // 1. check FEE operation
    check_fra_distribution_context_fee(tx).c(d!("invalid fee operation"))?;

    // 2. check `FraDistribution` operation
    if (0..2).any(|i| matches!(tx.body.operations[i], Operation::FraDistribution(_))) {
        Ok(())
    } else {
        Err(eg!())
    }
}

#[inline(always)]
fn check_fra_distribution_context_fee(tx: &Transaction) -> Result<()> {
    super::delegation::check_delegation_context_fee(tx, 2).c(d!())
}
