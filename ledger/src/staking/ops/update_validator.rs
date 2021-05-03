//!
//! # Update Validator Infomation
//!
//! update the verifier information at a certain block height
//! by using a multi-signature transaction.
//!

use crate::{
    data_model::NoReplayToken,
    staking::{cosig::CoSigOp, BlockHeight, Staking, Validator, ValidatorData},
};
use ruc::*;
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey};

/// Used as the inner object of a `UpdateValidator Operation`.
pub type UpdateValidatorOps = CoSigOp<Data>;

impl UpdateValidatorOps {
    /// Check the validity of an operation by running it in a staking simulator.
    #[inline(always)]
    pub fn check_run(&self, staking_simulator: &mut Staking) -> Result<()> {
        self.apply(staking_simulator).c(d!())
    }

    /// Apply new settings to the target `Staking` instance,
    /// will fail if existing info is found at the same height.
    pub fn apply(&self, staking: &mut Staking) -> Result<()> {
        self.verify(staking)
            .c(d!())
            .and_then(|_| self.check_context(staking).c(d!()))
            .and_then(|_| {
                staking
                    .validator_set_at_height(self.data.height, self.data.clone())
                    .c(d!())
            })
    }

    /// Apply new settings to the target `Staking` instance,
    /// ignore existing settings at the same height.
    #[inline(always)]
    pub fn apply_force(self, staking: &mut Staking) -> Result<()> {
        self.verify(staking)
            .c(d!())
            .and_then(|_| self.check_context(staking).c(d!()))
            .map(|_| staking.validator_set_at_height_force(self.data.height, self.data))
    }

    #[inline(always)]
    fn check_context(&self, staking: &Staking) -> Result<()> {
        if let Some(v) = staking.validator_get_current() {
            if self.data.height < v.height {
                return Err(eg!("invalid height"));
            }
        }
        Ok(())
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_related_pubkeys(&self) -> Vec<XfrPublicKey> {
        self.cosigs
            .keys()
            .chain(self.data.data.keys())
            .copied()
            .collect()
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new(
        kps: &[&XfrKeyPair],
        h: BlockHeight,
        v_set: Vec<Validator>,
        nonce: NoReplayToken,
    ) -> Result<Self> {
        Data::new(h, v_set)
            .c(d!())
            .map(|d| CoSigOp::create(d, nonce))
            .and_then(|mut op| op.batch_sign(kps).c(d!()).map(|_| op))
    }
}

/// The body of a `UpdateValidator Operation`.
type Data = ValidatorData;
