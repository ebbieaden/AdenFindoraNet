//!
//! # CoSignature
//!
//! Aka Multi-Signature, it is originally used to support `Governance` and `ValidatorUpdate`.
//!

use super::MAX_TOTAL_POWER;
use crate::staking::Staking;
use cryptohash::sha256::{self, Digest};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{self, Debug},
};
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey, XfrSignature};

/// A common structure for data with co-signatures.
///
/// **NOTE:**
/// - the nonce need not to be a primary type, eg. u128,
///     - actually we can take the first 16 bytes of SHA256("some no-replay bytes")
///     - eg. the first operation in a tx, usually the operation of paying fee
/// - the nonce will be checked in the ledger logic, not in this module.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(bound = "")]
pub struct CoSigOp<T>
where
    T: Debug + Serialize + for<'a> Deserialize<'a>,
{
    pub(crate) data: T,
    nonce: [u8; 16],
    pub(crate) cosigs: BTreeMap<XfrPublicKey, CoSig>,
}

impl<T> CoSigOp<T>
where
    T: Debug + Serialize + for<'a> Deserialize<'a>,
{
    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new(msg: T, nonce: [u8; 16]) -> Self {
        CoSigOp {
            data: msg,
            nonce,
            cosigs: BTreeMap::new(),
        }
    }

    /// Attach a new signature.
    #[inline(always)]
    pub fn sign(&mut self, kp: &XfrKeyPair) -> Result<()> {
        bincode::serialize(&(self.nonce, &self.data))
            .c(d!())
            .map(|msg| {
                let k = kp.get_pk();
                let v = CoSig::new(kp.get_pk(), kp.sign(&msg));
                self.cosigs.insert(k, v);
            })
    }

    /// Attach some new signatures in a batch mode.
    #[inline(always)]
    pub fn batch_sign(&mut self, kps: &[&XfrKeyPair]) -> Result<()> {
        let msg = bincode::serialize(&(self.nonce, &self.data)).c(d!())?;
        kps.iter().for_each(|kp| {
            let k = kp.get_pk();
            let v = CoSig::new(kp.get_pk(), kp.sign(&msg));
            self.cosigs.insert(k, v);
        });
        Ok(())
    }

    /// Check if a cosig is valid.
    pub fn check_cosigs(&self, rule: &CoSigRule) -> Result<()> {
        self.check_existence(rule)
            .c(d!())
            .and_then(|_| self.check_weight(rule).c(d!()))
            .and_then(|_| {
                let msg = bincode::serialize(&(self.nonce, &self.data)).c(d!())?;
                if self
                    .cosigs
                    .values()
                    .any(|sig| sig.pk.verify(&msg, &sig.sig).is_err())
                {
                    Err(eg!(CoSigErr::SigInvalid))
                } else {
                    Ok(())
                }
            })
    }

    #[inline(always)]
    fn check_existence(&self, rule: &CoSigRule) -> Result<()> {
        if self.cosigs.keys().any(|k| rule.weights.get(k).is_none()) {
            Err(eg!(CoSigErr::KeyUnknown))
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    fn check_weight(&self, rule: &CoSigRule) -> Result<()> {
        let rule_weights = rule
            .weights
            .values()
            .map(|v| v.weight as u128)
            .sum::<u128>();
        let actual_weights = self
            .cosigs
            .values()
            .flat_map(|s| rule.weights.get(&s.pk).map(|w| w.weight as u128))
            .sum::<u128>();

        if actual_weights * rule.threshold[1] < rule.threshold[0] * rule_weights {
            return Err(eg!(CoSigErr::WeightInsufficient));
        }

        Ok(())
    }

    /// Verify co-signatures based on current validators.
    pub fn verify(&self, staking: &Staking) -> Result<()> {
        staking
            .validator_get_current()
            .ok_or(eg!())
            .and_then(|vd| self.check_cosigs(vd.get_cosig_rule()).c(d!()))
    }

    /// Generate sha256 digest.
    #[inline(always)]
    pub fn hash(&self) -> Result<Digest> {
        bincode::serialize(self)
            .c(d!())
            .map(|bytes| sha256::hash(&bytes))
    }
}

/// The rule for a kind of data.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CoSigRule {
    // weight of each `XfrPublicKey`,
    weights: HashMap<XfrPublicKey, KeyWeight>,
    // check rule:
    // - `[actual weight].sum() / [rule weight].sum() >= threshold%`
    // - threshold% = `numerator / denominator` = `threshold[0] / threshold[1]`
    //
    // which equal to:
    // - `[actual weight].sum() * threshold[1] >= threshold[0] * [rule weight].sum()`
    // - convert to `i128` to avoid integer overflow
    threshold: [u128; 2],
}

impl CoSigRule {
    #[allow(missing_docs)]
    pub fn new(
        threshold: [u64; 2],
        mut weights: Vec<(XfrPublicKey, Weight)>,
    ) -> Result<Self> {
        let len = weights.len();
        weights.sort_by(|a, b| a.0.cmp(&b.0));
        weights.dedup_by(|a, b| a.0 == b.0);
        if len != weights.len() {
            return Err(eg!("found dup keys"));
        }

        if threshold[0] > threshold[1] || threshold[1] > MAX_TOTAL_POWER as u64 {
            return Err(eg!("invalid threshold"));
        }

        Ok(CoSigRule {
            weights: weights
                .into_iter()
                .map(|(pk, w)| (pk, KeyWeight::new(pk, w)))
                .collect(),
            threshold: [threshold[0] as u128, threshold[1] as u128],
        })
    }
}

type Weight = u64;

/// A pubkey and its co-reponding weight.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct KeyWeight {
    pk: XfrPublicKey,
    weight: Weight,
}

impl KeyWeight {
    #[inline(always)]
    fn new(pk: XfrPublicKey, weight: Weight) -> Self {
        KeyWeight { pk, weight }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct CoSig {
    pk: XfrPublicKey,
    sig: XfrSignature,
}

impl CoSig {
    #[inline(always)]
    fn new(pk: XfrPublicKey, sig: XfrSignature) -> Self {
        CoSig { pk, sig }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum CoSigErr {
    KeyUnknown,
    SigInvalid,
    WeightInsufficient,
}

impl fmt::Display for CoSigErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            CoSigErr::KeyUnknown => "found keys outside of the predefined rules",
            CoSigErr::WeightInsufficient => "total weight is lower than the threshold",
            CoSigErr::SigInvalid => "invalid signature",
        };
        write!(f, "{}", msg)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand_chacha::ChaChaRng;
    use rand_core::SeedableRng;

    #[derive(Default, Debug, Deserialize, Serialize)]
    struct Data {
        a: [i32; 12],
        b: [f32; 3],
        c: String,
        d: (),
    }

    fn gen_keypairs(n: u8) -> Vec<XfrKeyPair> {
        let mut prng = ChaChaRng::from_entropy();
        (0..n).map(|_| XfrKeyPair::generate(&mut prng)).collect()
    }

    #[test]
    fn staking_cosig() {
        let kps = gen_keypairs(100);
        let ws = kps.iter().map(|kp| (kp.get_pk(), 999)).collect::<Vec<_>>();

        assert!(CoSigRule::new([200, 100], ws.clone()).is_err());
        assert!(CoSigRule::new([200, 1 + MAX_TOTAL_POWER as u64], ws.clone()).is_err());

        // threshold: 75%
        let rule = pnk!(CoSigRule::new([75, 100], ws));

        let mut data = CoSigOp::new(Data::default(), 7_u128.to_le_bytes());
        pnk!(data.batch_sign(&kps.iter().skip(10).collect::<Vec<_>>()));
        assert!(data.check_cosigs(&rule).is_ok());

        kps.iter().skip(10).for_each(|kp| {
            pnk!(data.sign(kp));
        });
        assert!(data.check_cosigs(&rule).is_ok());

        data.data.a = [9; 12];
        assert!(data.check_cosigs(&rule).is_err());
        data.data.a = [0; 12];
        assert!(data.check_cosigs(&rule).is_ok());

        let mut data = CoSigOp::new(Data::default(), 8_u128.to_le_bytes());
        pnk!(data.batch_sign(&kps.iter().skip(25).collect::<Vec<_>>()));
        assert!(data.check_cosigs(&rule).is_ok());

        kps.iter().skip(25).for_each(|kp| {
            pnk!(data.sign(kp));
        });
        assert!(data.check_cosigs(&rule).is_ok());

        let mut data = CoSigOp::new(Data::default(), 9_u128.to_le_bytes());
        pnk!(data.batch_sign(&kps.iter().skip(45).collect::<Vec<_>>()));
        assert!(data.check_cosigs(&rule).is_err());

        kps.iter().skip(45).for_each(|kp| {
            pnk!(data.sign(kp));
        });
        assert!(data.check_cosigs(&rule).is_err());
    }
}
