//!
//! # On-Chain Governance
//!
//! propose a governance operation against some byzantine nodes
//! by using a multi-signature transaction.
//!
//! **NOTE**: always use the same multi-signature rules as `UpdateValidator`.
//!

use crate::{
    data_model::NoReplayToken,
    staking::{cosig::CoSigOp, Amount, Staking},
};
use lazy_static::lazy_static;
use ruc::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey};

lazy_static! {
    // TODO
    //
    // The current MVP version is a fixed rule,
    // and it will be upgraded to a mechanism
    // that can update rules by sending a specific transaction.
    static ref RULES: HashMap<ByzantineKind, Rule> = {
        map! {
            ByzantineKind::DuplicateVote => Rule::new(),
            ByzantineKind::LightClientAttack => Rule::new(),
            ByzantineKind::Unknown => Rule::new(),
        }
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
                    .governance_penalty_by_pubkey(
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

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new(
        kps: &[&XfrKeyPair],
        byzantine_id: XfrPublicKey,
        kind: ByzantineKind,
        nonce: NoReplayToken,
    ) -> Result<Self> {
        let mut op = CoSigOp::create(Data::new(kind, byzantine_id), nonce);
        op.batch_sign(kps).c(d!()).map(|_| op)
    }
}

/// Informances about a `Governance Operation`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    kind: ByzantineKind,
    byzantine_id: XfrPublicKey,
}

impl Data {
    #[inline(always)]
    fn new(kind: ByzantineKind, byzantine_id: XfrPublicKey) -> Self {
        Data { kind, byzantine_id }
    }
}

/// Kinds of byzantine behavior and corresponding punishment mechanism.
pub type RuleSet = HashMap<ByzantineKind, Rule>;

/// Kinds of byzantine behaviors:
/// - `DuplicateVote` and `LightClientAttack` can be auto-detected by tendermint
/// - other attack kinds need to be defined and applied on the application side
#[non_exhaustive]
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ByzantineKind {
    DuplicateVote,
    LightClientAttack,
    Unknown,
}

/// Punishment mechanism for each kind of byzantine behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    // TODO
}

impl Rule {
    fn new() -> Self {
        Rule {}
    }

    /// Calculate punishment amount according to the corresponding rule.
    ///
    /// Currently We just set the amount to `i64::MAX` which means
    /// all investment income of the byzantine node will be punished,
    /// and its vote power will be decreased to zero which means
    /// it can not become a formal on-line validator anymore.
    pub fn gen_penalty_amount(&self, _byzantine_id: &XfrPublicKey) -> Amount {
        // TODO
        i64::MAX
    }
}
