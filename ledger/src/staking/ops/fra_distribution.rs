//!
//! # FRA Distribution
//!
//! Used to support the distribution of the official token FRA.
//!

use crate::{
    data_model::{Operation, Transaction},
    staking::{cosig::CoSigOp, Staking},
};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zei::xfr::sig::XfrPublicKey;

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
            .chain(self.data.allocation_table.keys())
            .copied()
            .collect()
    }
}

/// The body of a `FraDistribution Operation`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    uuid: u64,
    /// How many FRAs to pay for each address.
    pub allocation_table: BTreeMap<XfrPublicKey, u64>,
}

// Check tx and return the amount of delegation.
// - total amount of operations is 2
// - the first one is a `TransferAsset` to pay fee
// - the second one is a `FraDistribution`
fn check_fra_distribution_context(tx: &Transaction) -> Result<()> {
    if 2 != tx.body.operations.len() {
        return Err(eg!("incorrect number of operations"));
    }

    // 1. the first operation must be a FEE operation
    check_fra_distribution_context_fee(tx).c(d!("invalid fee operation"))?;

    // 2. the second operation must be a `FraDistribution` operation
    if let Operation::FraDistribution(_) = tx.body.operations[1] {
        Ok(())
    } else {
        Err(eg!())
    }
}

#[inline(always)]
fn check_fra_distribution_context_fee(tx: &Transaction) -> Result<()> {
    super::delegation::check_delegation_context_fee(tx).c(d!())
}
