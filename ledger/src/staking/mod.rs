//!
//! # Staking
//!
//! - manage validator information
//! - manage delegation information
//! - manage the distribution of investment income
//! - manage on-chain governance
//!

#![deny(warnings)]
#![deny(missing_docs)]

pub mod cosig;
mod init;
pub mod ops;

use super::data_model::FRA_DECIMALS;
use cosig::CoSigRule;
use cryptohash::sha256::{self, Digest};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use zei::xfr::sig::XfrPublicKey;

/// Staking entry
///
/// Init:
/// 1. set_custom_height
/// 2. set_validators_at_height
///
/// Usage:
/// - change_power ...
/// - apply_validators_at_height
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Staking {
    // the main logic when updating:
    // - the new validator inherits the original vote power, if any
    // - all delegate addresss locked on those outdated validators will be unlocked
    // immediately, and the related delegate income will also be settled immediately
    vi: ValidatorInfo,
    // all assets owned by these addrs are NOT permitted to be transfered out,
    // but receiving assets from outer addrs is permitted.
    //
    // when the end-time of delegations arrived,
    // we will try to paid the rewards until all is successful.
    di: DelegationInfo,
    // current block height in the context of tendermint.
    cur_height: BlockHeight,
}

impl Staking {
    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new() -> Self {
        let vd = pnk!(init::get_inital_validators());
        let cur_height = vd.height;
        Staking {
            vi: map! {B cur_height => vd },
            di: DelegationInfo::default(),
            cur_height,
        }
    }

    /// Get the validators that exactly be setted at a specified height.
    #[inline(always)]
    pub fn get_validators_at_height(&self, h: BlockHeight) -> Option<Vec<&Validator>> {
        self.vi.get(&h).map(|v| v.data.values().collect())
    }

    /// Check if there is some settings on a specified height.
    #[inline(always)]
    pub fn has_validator_settings_at_height(&self, h: BlockHeight) -> bool {
        self.vi.contains_key(&h)
    }

    /// Set the validators that will be used for the specified height.
    #[inline(always)]
    pub fn set_validators_at_height(
        &mut self,
        h: BlockHeight,
        v: ValidatorData,
    ) -> Result<()> {
        if self.vi.get(&h).is_some() {
            Err(eg!("already exists"))
        } else {
            self.set_validators_at_height_force(h, v);
            Ok(())
        }
    }

    /// Set the validators that will be used for the specified height,
    /// no matter if there is an existing set of validators at that height.
    #[inline(always)]
    pub fn set_validators_at_height_force(&mut self, h: BlockHeight, v: ValidatorData) {
        self.vi.insert(h, v);
    }

    /// Get the validators that will be used for the specified height.
    #[inline(always)]
    pub fn get_validators_effective_at_height(
        &self,
        h: BlockHeight,
    ) -> Option<&ValidatorData> {
        self.vi.range(0..=h).last().map(|(_, v)| v)
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_current_validators(&self) -> Option<&ValidatorData> {
        let h = self.cur_height;
        self.vi.range(0..=h).last().map(|(_, v)| v)
    }

    /// Remove the validators that will be used for the specified height.
    #[inline(always)]
    pub fn remove_validators_at_height(
        &mut self,
        h: BlockHeight,
    ) -> Result<Vec<Validator>> {
        self.vi
            .remove(&h)
            .map(|v| v.data.into_iter().map(|(_, v)| v).collect())
            .ok_or(eg!("not exists"))
    }

    /// Get the validators that will be used for the specified height.
    #[inline(always)]
    pub fn get_validators_effective_at_height_mut(
        &mut self,
        h: BlockHeight,
    ) -> Option<&mut ValidatorData> {
        self.vi.range_mut(0..=h).last().map(|(_, v)| v)
    }

    /// Make the validators at the specified height to be effective.
    pub fn apply_validators_at_height(&mut self, h: BlockHeight) -> Result<()> {
        let prev = self.vi.range(0..h).last().map(|(_, v)| (*v).clone());

        // copy the power of the previous term
        if let Some(prev) = prev {
            self.vi.get_mut(&h).ok_or(eg!("not exists")).map(|vs| {
                vs.data.iter_mut().for_each(|(k, v)| {
                    if let Some(pv) = prev.data.get(k) {
                        v.td_power = pv.td_power;
                    }
                });
            })?;
        }

        // set new height after all is well
        self.cur_height = h;

        // clean old data
        self.clean_validators_before_height(h);

        Ok(())
    }

    // Clean validator-info older than the specified height.
    #[inline(always)]
    fn clean_validators_before_height(&mut self, h: BlockHeight) {
        self.vi = self.vi.split_off(&h);
    }

    /// increase/decrease vote power of a specified validator.
    fn change_power(&mut self, validator: &XfrPublicKey, power: i64) -> Result<()> {
        self.check_power(power)
            .c(d!())
            .and_then(|_| {
                self.get_validators_effective_at_height_mut(self.cur_height)
                    .ok_or(eg!())
            })
            .and_then(|cur| {
                cur.data
                    .get_mut(validator)
                    .map(|v| {
                        v.td_power += power;
                    })
                    .ok_or(eg!())
            })
    }

    #[inline(always)]
    fn check_power(&self, new_power: i64) -> Result<()> {
        if self.total_power() + new_power > MAX_TOTAL_POWER {
            Err(eg!("total power overflow"))
        } else {
            Ok(())
        }
    }

    // calculate current total vote-power
    #[inline(always)]
    fn total_power(&self) -> i64 {
        self.get_validators_effective_at_height(self.cur_height)
            .map(|vs| vs.data.values().map(|v| v.td_power).sum())
            .unwrap_or(0)
    }

    /// Set a custom block height,
    /// used in some initial opertions.
    #[inline(always)]
    pub fn set_custom_height(&mut self, h: BlockHeight) {
        self.cur_height = h;
    }

    /// Start a new delegation.
    /// - increase the vote power of the co-responding validator
    ///
    /// Validator must do self-delegatation first,
    /// and its delegation end_height must be `i64::MAX`.
    ///
    /// **NOTE:** It is the caller's duty to ensure that
    /// there is enough FRAs existing in the target address(owner).
    pub fn delegate(
        &mut self,
        owner: XfrPublicKey,
        validator: XfrPublicKey,
        am: Amount,
        start_height: BlockHeight,
        mut end_height: BlockHeight,
    ) -> Result<()> {
        if !(MIN_DELEGATION_AMOUNT..=MAX_DELEGATION_AMOUNT).contains(&(am as u64)) {
            return Err(eg!("invalid delegation amount"));
        }

        if let Some(d) = self.get_delegation(&validator) {
            // check delegation deadline
            if i64::MAX != d.end_height {
                // should NOT happen
                return Err(eg!("invalid self-delegation of validator"));
            }
        } else if owner == validator {
            // do self-delegation
            end_height = i64::MAX;
        } else {
            return Err(eg!("self-delegation has not been finished"));
        }

        let k = owner;
        if self.di.addr_map.get(&k).is_some() {
            return Err(eg!("already exists"));
        }

        self.change_power(&validator, am as i64).c(d!())?;

        let v = Delegation {
            amount: am,
            validator,
            rwd_pk: owner,
            start_height,
            end_height,
            state: DelegationState::Locked,
            rwd_amount: 0,
        };

        self.di.addr_map.insert(k, v);
        self.di
            .end_height_map
            .entry(end_height)
            .or_insert_with(HashSet::new)
            .insert(k);

        Ok(())
    }

    /// When delegation period expired,
    /// - compute rewards
    /// - decrease the vote power of the co-responding validator
    ///
    /// **NOTE:** validator self-undelegation is not permitted
    pub fn undelegate(&mut self, addr: &XfrPublicKey) -> Result<()> {
        let h = self.cur_height;
        let mut orig_h = None;

        if let Some(vs) = self.get_validators_effective_at_height(h) {
            if vs.data.get(addr).is_some() {
                return Err(eg!("validator self-undelegation is not permitted"));
            }
        }

        let (validator, power) = self
            .di
            .addr_map
            .get_mut(addr)
            .ok_or(eg!("not exists"))
            .map(|d| {
                if d.end_height != h {
                    orig_h = Some(d.end_height);
                    d.end_height = h;
                }
                d.compute_rewards();
                (d.validator, -(d.amount as i64))
            })?;

        // scene: forced un-delegation
        if let Some(orig_h) = orig_h {
            self.di
                .end_height_map
                .get_mut(&orig_h)
                .map(|set| set.remove(addr));
            self.di
                .end_height_map
                .entry(h)
                .or_insert_with(HashSet::new)
                .insert(addr.to_owned());
        }

        self.change_power(&validator, power).c(d!())
    }

    #[inline(always)]
    fn unfrozen_delegation(&mut self, addr: &XfrPublicKey) -> Result<Delegation> {
        let d = self.di.addr_map.remove(addr).ok_or(eg!("not exists"))?;
        if d.state == DelegationState::Paid {
            Ok(d)
        } else {
            // we assume that this probability is very low
            self.di.addr_map.insert(addr.to_owned(), d);
            Err(eg!("unpaid delegation"))
        }
    }

    /// Expand delegation scale
    ///
    /// **NOTE:** It is the caller's duty to ensure that
    /// there is enough FRAs existing in the target address(owner).
    pub fn extend_delegation(
        &mut self,
        owner: &XfrPublicKey,
        am: Option<Amount>,
        end_height: Option<BlockHeight>,
    ) -> Result<()> {
        let addr = owner;
        let d = if let Some(d) = self.di.addr_map.get_mut(addr) {
            d
        } else {
            return Err(eg!("not exists"));
        };

        if let Some(am) = am {
            if am > d.amount {
                d.amount = am;
            } else {
                return Err(eg!("new amount must be bigger than the old one"));
            }
        }

        if let Some(h) = end_height {
            if h > d.end_height {
                let orig_h = d.end_height;
                d.end_height = h;
                self.di
                    .end_height_map
                    .get_mut(&orig_h)
                    .ok_or(eg!())?
                    .remove(addr);
                self.di
                    .end_height_map
                    .entry(h)
                    .or_insert_with(HashSet::new)
                    .insert(addr.to_owned());
            } else {
                return Err(eg!("new end_height must be bigger than the old one"));
            }
        }

        Ok(())
    }

    /// Get the delegation instance of `addr`.
    #[inline(always)]
    pub fn get_delegation(&self, addr: &XfrPublicKey) -> Option<&Delegation> {
        self.di.addr_map.get(&addr)
    }

    /// Check if the `addr` is in a state of delegation
    #[inline(always)]
    pub fn in_delegation(&self, addr: &XfrPublicKey) -> bool {
        self.di.addr_map.get(&addr).is_some()
    }

    /// Query delegation rewards before a specified height(included).
    #[inline(always)]
    pub fn get_delegation_rewards_before_height(
        &self,
        h: BlockHeight,
    ) -> Option<Vec<DelegationReward>> {
        self.di.end_height_map.get(&h).map(|addrs| {
            addrs
                .iter()
                .flat_map(|addr| self.di.addr_map.get(addr).map(|d| d.into()))
                .collect()
        })
    }

    /// call this when:
    /// - the frozen period expired
    /// - rewards have been paid successfully.
    pub fn clean_finished_delegation(&mut self) -> Result<()> {
        let h = self.cur_height - FROZEN_BLOCK_CNT;
        if 0 < h {
            self.clean_finished_delegation_before_height(h);
            Ok(())
        } else {
            Err(eg!("block height is too small"))
        }
    }

    /// @param h: included
    pub fn clean_finished_delegation_before_height(&mut self, h: BlockHeight) {
        self.di
            .end_height_map
            .range(0..=h)
            .map(|(k, v)| (k.to_owned(), (*v).clone()))
            .collect::<Vec<_>>()
            .iter()
            .for_each(|(h, addrs)| {
                addrs.iter().for_each(|addr| {
                    if self.unfrozen_delegation(addr).is_ok() {
                        self.di
                            .end_height_map
                            .get_mut(&h)
                            .map(|addrs| addrs.remove(addr));
                    }
                });
                // this unwrap is safe
                if self.di.end_height_map.get(&h).unwrap().is_empty() {
                    self.di.end_height_map.remove(&h);
                }
            });
    }

    /// Penalize the FRAs of a specified address.
    #[inline(always)]
    pub fn governance_penalty(&mut self, addr: &XfrPublicKey, am: Amount) -> Result<()> {
        if am <= 0 {
            return Err(eg!("the amount must be a positive integer"));
        }
        self.import_extern_amount(addr, -am).c(d!())
    }

    /// Import extern amount changes, eg.. 'Block Rewards'/'Governance Penalty'
    pub fn import_extern_amount(
        &mut self,
        addr: &XfrPublicKey,
        am: Amount,
    ) -> Result<()> {
        let d = if let Some(d) = self.di.addr_map.get_mut(addr) {
            d
        } else {
            return Err(eg!("not exists"));
        };

        if DelegationState::Paid == d.state {
            return Err(eg!("delegation has been paid"));
        } else {
            d.rwd_amount = d.rwd_amount.saturating_add(am);
        }

        Ok(())
    }

    /// Generate sha256 digest.
    #[inline(always)]
    pub fn hash(&self) -> Result<Digest> {
        bincode::serialize(self)
            .c(d!())
            .map(|bytes| sha256::hash(&bytes))
    }
}

const FRA: u64 = 10_u64.pow(FRA_DECIMALS as u32);
const MIN_DELEGATION_AMOUNT: u64 = 32 * FRA;
const MAX_DELEGATION_AMOUNT: u64 = 32_0000 * FRA;

// A limitation from
// [tendermint](https://docs.tendermint.com/v0.33/spec/abci/apps.html#validator-updates)
//
// > Note that the maximum total power of the validator set
// > is bounded by MaxTotalVotingPower = MaxInt64 / 8.
// > Applications are responsible for ensuring
// > they do not make changes to the validator set
// > that cause it to exceed this limit.
const MAX_TOTAL_POWER: i64 = i64::MAX / 8;

// Block time interval, in seconds.
const BLOCK_INTERVAL: i64 = 15;

/// The lock time after the delegation expires, about 30 days.
pub const FROZEN_BLOCK_CNT: i64 = 3600 * 24 * 30 / BLOCK_INTERVAL;

// used to express some descriptive information
type Memo = String;

// block height of tendermint
pub(crate) type BlockHeight = i64;

// use i64 to keep compatible with the logic of asset penalty
type Amount = i64;

type ValidatorInfo = BTreeMap<BlockHeight, ValidatorData>;

/// Data of the effective validators on a specified height.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorData {
    pub(crate) height: BlockHeight,
    pub(crate) cosig_rule: CoSigRule,
    /// Major data of validators.
    pub data: HashMap<XfrPublicKey, Validator>,
}

impl ValidatorData {
    #[allow(missing_docs)]
    pub fn new(h: BlockHeight, v_set: Vec<Validator>) -> Result<Self> {
        if h < 1 {
            return Err(eg!("invalid start height"));
        }

        let mut vs = map! {};
        for v in v_set.into_iter() {
            if vs.insert(v.id, v).is_some() {
                return Err(eg!("duplicate entries"));
            }
        }

        let cosig_rule = Self::gen_cosig_rule(vs.keys().copied().collect()).c(d!())?;

        Ok(ValidatorData {
            height: h,
            cosig_rule,
            data: vs,
        })
    }

    // When updating the validator list, all validators have equal weights.
    fn gen_cosig_rule(validator_ids: Vec<XfrPublicKey>) -> Result<CoSigRule> {
        // The minimum weight threshold required
        // when updating validator information, 80%.
        const COSIG_THRESHOLD: [u64; 2] = [4, 5];

        if 3 > validator_ids.len() {
            return Err(eg!("too few validators"));
        }
        CoSigRule::new(
            COSIG_THRESHOLD,
            validator_ids.into_iter().map(|v| (v, 1)).collect(),
        )
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_cosig_rule(&self) -> &CoSigRule {
        &self.cosig_rule
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_validators(&self) -> &HashMap<XfrPublicKey, Validator> {
        &self.data
    }
}

// the same address is not allowed to delegate twice at the same time,
// so it is feasible to use `XfrPublicKey` as the map key.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
struct DelegationInfo {
    addr_map: HashMap<XfrPublicKey, Delegation>,
    end_height_map: BTreeMap<BlockHeight, HashSet<XfrPublicKey>>,
}

/// Validator info
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Validator {
    /// pubkey in the context of tendermint
    pub td_pubkey: Vec<u8>,
    /// vote power in the context of Staking
    pub td_power: i64,
    /// public key of validator, aka 'Validator ID'.
    ///
    /// staking rewards will be paid to this addr
    /// - eg.. self-delegation rewards
    /// - eg.. block rewards
    pub id: XfrPublicKey,
    /// optional descriptive information
    pub memo: Memo,
}

impl Validator {
    #[allow(missing_docs)]
    pub fn new(td_pubkey: Vec<u8>, td_power: i64, id: XfrPublicKey, memo: Memo) -> Self {
        Validator {
            td_pubkey,
            td_power,
            id,
            memo,
        }
    }
}

/// FRA delegation, include:
/// - user delegation
/// - validator's self-delegation
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Delegation {
    /// total `NonConfidential` FRAs in a staking address
    pub amount: Amount,
    /// the target validator to delegated to
    pub validator: XfrPublicKey,
    /// delegation rewards will be paid to this pk
    pub rwd_pk: XfrPublicKey,
    /// the height at which the delegation starts
    pub start_height: BlockHeight,
    /// the height at which the delegation ends
    ///
    /// **NOTE:** before users can actually get the rewards,
    /// they need to wait for an extra `FROZEN_BLOCK_CNT` period
    pub end_height: BlockHeight,
    #[allow(missing_docs)]
    pub state: DelegationState,
    /// set this field when `Locked` state finished
    pub rwd_amount: Amount,
}

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DelegationState {
    /// during delegation
    Locked,
    /// delegation finished,
    /// entered frozen time
    Frozen,
    /// during or after frozen time,
    /// and rewards have been paid successfully,
    /// the co-responding account should be unfrozen
    Paid,
}

impl Default for DelegationState {
    fn default() -> Self {
        DelegationState::Locked
    }
}

impl Delegation {
    /// calculate the amount(in FRA units) that
    /// should be paid to the owner of this delegation
    ///
    /// > **NOTE:**
    /// > use 'AssignAdd' instead of 'Assign'
    /// > to keep compatible with the logic of asset penalty.
    ///
    /// > **TODO:** implement the real logic.
    fn compute_rewards(&mut self) {
        let n = 0;
        self.rwd_amount += n;
    }
}

#[allow(missing_docs)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DelegationReward {
    /// the receiver of this reward
    pub id: XfrPublicKey,
    /// the amount of this reward
    pub am: Amount,
}

impl DelegationReward {
    #[inline(always)]
    fn new(id: XfrPublicKey, am: Amount) -> Self {
        DelegationReward { id, am }
    }
}

impl From<&Delegation> for DelegationReward {
    fn from(d: &Delegation) -> Self {
        DelegationReward::new(d.rwd_pk, d.rwd_amount)
    }
}

#[cfg(test)]
mod test {
    // TODO
}
