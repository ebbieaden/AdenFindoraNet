//!
//! # On-Chain Governance
//!
//! propose a governance operation against some byzantine nodes
//! by using a multi-signature transaction.
//!
//! **NOTE**: always use the same multi-signature rules as `UpdateValidator`.
//!

use crate::staking::{cosig::CoSigOp, Amount, Staking};
use lazy_static::lazy_static;
use ruc::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zei::xfr::sig::XfrPublicKey;

lazy_static! {
    // TODO
    //
    // The current MVP version is a fixed rule,
    // and it will be upgraded to a mechanism
    // that can update rules by sending a specific transaction.
    static ref RULES: HashMap<ByzantineKind, Rule> = {
        map! { ByzantineKind::SignMultiBlocks => Rule::new() }
    };
}

/// Used as the inner object of a `Governance Operation`.
pub type GovernanceOps = CoSigOp<Data>;

impl GovernanceOps {
    /// Check the validity of an operation by running it in a staking simulator.
    #[inline(always)]
    pub fn check_run(&self, staking_simulator: &mut Staking) -> Result<()> {
        self.apply(staking_simulator).c(d!())
    }

    /// Apply new governance to the target `Staking` instance.
    pub fn apply(&self, staking: &mut Staking) -> Result<()> {
        self.verify(staking)
            .c(d!())
            .and_then(|_| RULES.get(&self.data.kind).ok_or(eg!()))
            .and_then(|rule| {
                staking
                    .governance_penalty(
                        &self.data.byzantine_id,
                        rule.gen_penalty_amount(&self.data.byzantine_id),
                    )
                    .c(d!())
            })
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_related_pubkeys(&self) -> Vec<XfrPublicKey> {
        self.cosigs
            .keys()
            .chain([self.data.byzantine_id].iter())
            .copied()
            .collect()
    }
}

/// Informances about a `Governance Operation`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    kind: ByzantineKind,
    byzantine_id: XfrPublicKey,
}

/// Kinds of byzantine behavior and corresponding punishment mechanism.
pub type RuleSet = HashMap<ByzantineKind, Rule>;

/// **TODO**
///
/// Kinds of byzantine behaviors.
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ByzantineKind {
    SignMultiBlocks,
    // TODO
}

/// **TODO**
///
/// Punishment mechanism for a kind of byzantine behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    // TODO
}

impl Rule {
    fn new() -> Self {
        Rule {}
    }

    /// **TODO**
    ///
    /// Calculate the amount of FRA punishment
    /// according to the corresponding rule.
    pub fn gen_penalty_amount(&self, byzantine_id: &XfrPublicKey) -> Amount {
        let _ = byzantine_id;
        1
    }
}
