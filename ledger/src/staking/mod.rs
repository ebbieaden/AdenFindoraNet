//!
//! # Staking
//!
//! - manage validator information
//! - manage delegation information
//! - manage the distribution of investment income
//! - manage on-chain governance
//! - manage the official re-distribution of FRA
//!

#![deny(warnings)]
#![deny(missing_docs)]

pub mod cosig;
mod init;
pub mod ops;

use crate::{
    data_model::{
        Operation, Transaction, TransferAsset, TxoSID, ASSET_TYPE_FRA, FRA_DECIMALS,
    },
    store::LedgerStatus,
};
use cosig::CoSigRule;
use cryptohash::sha256::{self, Digest};
use ops::fra_distribution::FraDistributionOps;
use ruc::*;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    mem,
};
use zei::xfr::{
    sig::{XfrKeyPair, XfrPublicKey},
    structs::{XfrAmount, XfrAssetType},
};

/// Staking entry
///
/// Init:
/// 1. set_custom_block_height
/// 2. validator_set_at_height
///
/// Usage:
/// - validator_change_power ...
/// - validator_apply_at_height
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
    // FRA CoinBase.
    coinbase: CoinBase,
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
            coinbase: CoinBase::gen(),
        }
    }

    /// Get the validators that exactly be setted at a specified height.
    #[inline(always)]
    pub fn validator_get_at_height(&self, h: BlockHeight) -> Option<Vec<&Validator>> {
        self.vi.get(&h).map(|v| v.data.values().collect())
    }

    // Check if there is some settings on a specified height.
    #[inline(always)]
    fn validator_has_settings_at_height(&self, h: BlockHeight) -> bool {
        self.vi.contains_key(&h)
    }

    /// Set the validators that will be used for the specified height.
    #[inline(always)]
    pub fn validator_set_at_height(
        &mut self,
        h: BlockHeight,
        v: ValidatorData,
    ) -> Result<()> {
        if self.validator_has_settings_at_height(h) {
            Err(eg!("already exists"))
        } else {
            self.validator_set_at_height_force(h, v);
            Ok(())
        }
    }

    /// Set the validators that will be used for the specified height,
    /// no matter if there is an existing set of validators at that height.
    #[inline(always)]
    pub fn validator_set_at_height_force(&mut self, h: BlockHeight, v: ValidatorData) {
        self.vi.insert(h, v);
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn validator_get_current(&self) -> Option<&ValidatorData> {
        self.validator_get_effective_at_height(self.cur_height)
    }

    /// Get the validators that will be used for the specified height.
    #[inline(always)]
    pub fn validator_get_effective_at_height(
        &self,
        h: BlockHeight,
    ) -> Option<&ValidatorData> {
        self.vi.range(0..=h).last().map(|(_, v)| v)
    }

    /// Remove the validators that will be used for the specified height.
    #[inline(always)]
    pub fn validator_remove_at_height(
        &mut self,
        h: BlockHeight,
    ) -> Result<Vec<Validator>> {
        self.vi
            .remove(&h)
            .map(|v| v.data.into_iter().map(|(_, v)| v).collect())
            .ok_or(eg!("not exists"))
    }

    /// Get the validators that will be used for a specified height.
    #[inline(always)]
    pub fn validator_get_effective_at_height_mut(
        &mut self,
        h: BlockHeight,
    ) -> Option<&mut ValidatorData> {
        self.vi.range_mut(0..=h).last().map(|(_, v)| v)
    }

    /// Get the validators exactly on a specified height.
    #[inline(always)]
    pub fn validator_get_at_height_mut(
        &mut self,
        h: BlockHeight,
    ) -> Option<&mut ValidatorData> {
        self.vi.get_mut(&h)
    }

    /// Make the validators at current height to be effective.
    #[inline(always)]
    pub fn validator_apply_current(&mut self) {
        let h = self.cur_height;
        self.validator_apply_at_height(h);

        // clean old data before current height
        self.validator_clean_before_height(h);
    }

    /// Make the validators at a specified height to be effective.
    pub fn validator_apply_at_height(&mut self, h: BlockHeight) {
        let prev = self.validator_get_effective_at_height(h - 1).cloned();

        if let Some(prev) = prev {
            if let Some(vs) = self.validator_get_at_height_mut(h) {
                // inherit the powers of previous settings
                // if new settings were found
                vs.data.iter_mut().for_each(|(k, v)| {
                    if let Some(pv) = prev.data.get(k) {
                        v.td_power = pv.td_power;
                    }
                });
            } else {
                // copy previous settings
                // if new settings were not found.
                self.validator_set_at_height_force(h, prev);
            }
        }
    }

    // Clean validator-info older than the specified height.
    #[inline(always)]
    fn validator_clean_before_height(&mut self, h: BlockHeight) {
        self.vi = self.vi.split_off(&h);
    }

    /// increase/decrease vote power of a specified validator.
    fn validator_change_power(
        &mut self,
        validator: &XfrPublicKey,
        power: i64,
    ) -> Result<()> {
        self.validator_check_power(power, validator)
            .c(d!())
            .and_then(|_| {
                self.validator_get_effective_at_height_mut(self.cur_height)
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
    fn validator_check_power(
        &self,
        new_power: i64,
        vldtor: &XfrPublicKey,
    ) -> Result<()> {
        let total_power = self.validator_total_power() + new_power;
        if MAX_TOTAL_POWER < total_power {
            return Err(eg!("total power overflow"));
        }

        if let Some(v) = self
            .validator_get_current()
            .and_then(|vd| vd.data.get(vldtor))
        {
            if ((v.td_power + new_power) as u128)
                .checked_mul(MAX_POWER_PERCENT_PER_VALIDATOR[1])
                .ok_or(eg!())?
                > MAX_POWER_PERCENT_PER_VALIDATOR[0]
                    .checked_mul(total_power as u128)
                    .ok_or(eg!())?
            {
                return Err(eg!("validator power overflow"));
            }
        }

        Ok(())
    }

    // calculate current total vote-power
    #[inline(always)]
    fn validator_total_power(&self) -> i64 {
        self.validator_get_effective_at_height(self.cur_height)
            .map(|vs| vs.data.values().map(|v| v.td_power).sum())
            .unwrap_or(0)
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn set_custom_block_height(&mut self, h: BlockHeight) {
        self.cur_height = h;
    }

    /// Start a new delegation.
    /// - increase the vote power of the co-responding validator
    /// - self-delegation will generate 10x power than user-delegation
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
        const POWER_MUL: i64 = 10;
        if !(MIN_DELEGATION_AMOUNT..=MAX_DELEGATION_AMOUNT).contains(&(am as u64)) {
            return Err(eg!("invalid delegation amount"));
        }

        let mut power = am as i64 / POWER_MUL;

        if let Some(d) = self.delegation_get(&validator) {
            // check delegation deadline
            if u64::MAX != d.end_height {
                // should NOT happen
                return Err(eg!("invalid self-delegation of validator"));
            }
        } else if owner == validator {
            // do self-delegation
            end_height = u64::MAX;
            power *= POWER_MUL;
        } else {
            return Err(eg!("self-delegation has not been finished"));
        }

        let k = owner;
        if self.di.addr_map.get(&k).is_some() {
            return Err(eg!("already exists"));
        }

        self.validator_change_power(&validator, power).c(d!())?;

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
            .or_insert_with(BTreeSet::new)
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

        if let Some(vs) = self.validator_get_effective_at_height(h) {
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
                .or_insert_with(BTreeSet::new)
                .insert(addr.to_owned());
        }

        self.validator_change_power(&validator, power).c(d!())
    }

    #[inline(always)]
    fn delegation_unfrozen(&mut self, addr: &XfrPublicKey) -> Result<Delegation> {
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
    pub fn delegation_extend(
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
                    .or_insert_with(BTreeSet::new)
                    .insert(addr.to_owned());
            } else {
                return Err(eg!("new end_height must be bigger than the old one"));
            }
        }

        Ok(())
    }

    /// Get the delegation instance of `addr`.
    #[inline(always)]
    pub fn delegation_get(&self, addr: &XfrPublicKey) -> Option<&Delegation> {
        self.di.addr_map.get(&addr)
    }

    /// Get the delegation instance of `addr`.
    #[inline(always)]
    pub fn delegation_get_mut(
        &mut self,
        addr: &XfrPublicKey,
    ) -> Option<&mut Delegation> {
        self.di.addr_map.get_mut(&addr)
    }

    /// Check if the `addr` is in a state of delegation
    #[inline(always)]
    pub fn delegation_has_addr(&self, addr: &XfrPublicKey) -> bool {
        self.di.addr_map.contains_key(&addr)
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn delegation_get_rewards(&self) -> Vec<DelegationReward> {
        self.delegation_get_frozens()
            .into_iter()
            .map(|d| d.into())
            .collect()
    }

    /// Query delegation rewards before a specified height(included).
    #[inline(always)]
    pub fn delegation_get_rewards_before_height(
        &self,
        h: BlockHeight,
    ) -> Vec<DelegationReward> {
        self.delegation_get_frozens_before_height(h)
            .into_iter()
            .map(|d| d.into())
            .collect()
    }

    /// Query all frozen delegations.
    #[inline(always)]
    pub fn delegation_get_frozens(&self) -> Vec<&Delegation> {
        self.delegation_get_frozens_before_height(self.cur_height)
    }

    /// Query frozen delegations before a specified height(included).
    #[inline(always)]
    pub fn delegation_get_frozens_before_height(
        &self,
        h: BlockHeight,
    ) -> Vec<&Delegation> {
        self.di
            .end_height_map
            .range(..=h)
            .flat_map(|(_, addrs)| {
                addrs
                    .iter()
                    .flat_map(|addr| self.di.addr_map.get(addr))
                    .filter(|d| matches!(d.state, DelegationState::Frozen))
            })
            .collect()
    }

    /// Clean delegation states along with each new block.
    pub fn deletation_process(&mut self) {
        let h = self.cur_height - FROZEN_BLOCK_CNT;

        if 0 < h {
            self.di
                .end_height_map
                .range(..=h)
                .map(|(_, addr)| addr)
                .flatten()
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|addr| {
                    if let Some(d) = self.di.addr_map.get_mut(&addr) {
                        if DelegationState::Locked == d.state {
                            d.state = DelegationState::Frozen;
                        }
                    }
                });

            self.deletation_process_finished_before_height(h);
        }
    }

    // call this when:
    // - the frozen period expired
    // - rewards have been paid successfully.
    //
    // @param h: included
    fn deletation_process_finished_before_height(&mut self, h: BlockHeight) {
        self.di
            .end_height_map
            .range(0..=h)
            .map(|(k, v)| (k.to_owned(), (*v).clone()))
            .collect::<Vec<_>>()
            .iter()
            .for_each(|(h, addrs)| {
                addrs.iter().for_each(|addr| {
                    if self.delegation_unfrozen(addr).is_ok() {
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
        self.delegation_import_extern_amount(addr, -am).c(d!())
    }

    /// A helper for setting block rewards in ABCI.
    pub fn set_block_rewards(
        &mut self,
        addr: TendermintAddrRef,
        am: Amount,
    ) -> Result<()> {
        let pk = self
            .validator_get_current()
            .and_then(|vd| vd.addr_td_to_app.get(addr).copied())
            .ok_or(eg!())?;

        if self.addr_is_validator(&pk) {
            return Err(eg!("not validator"));
        }

        self.delegation_import_extern_amount(&pk, am).c(d!())
    }

    /// Import extern amount changes,
    /// eg.. 'Block Rewards'/'Governance Penalty'
    pub fn delegation_import_extern_amount(
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

        // extern changes can NOT increase vote power
        if 0 > am && self.addr_is_validator(addr) {
            self.validator_change_power(addr, am).c(d!())?;
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

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn coinbase_pubkey(&self) -> XfrPublicKey {
        self.coinbase.pubkey
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn coinbase_keypair(&self) -> &XfrKeyPair {
        &self.coinbase.keypair
    }

    /// Add new FRA utxo to CoinBase.
    #[inline(always)]
    pub fn coinbase_recharge(&mut self, txo_sid: TxoSID) {
        self.coinbase.bank.insert(txo_sid);
    }

    /// Get all avaliable utos owned by CoinBase.
    #[inline(always)]
    pub fn coinbase_txos(&mut self) -> BTreeSet<TxoSID> {
        self.coinbase.bank.clone()
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn coinbase_clean_spent_txos(&mut self, ls: &LedgerStatus) {
        self.coinbase.bank.clone().into_iter().for_each(|sid| {
            if !ls.is_unspent_txo(sid) {
                self.coinbase.bank.remove(&sid);
            }
        });
    }

    /// Add new fra distribution plan.
    pub fn coinbase_config_fra_distribution(
        &mut self,
        ops: FraDistributionOps,
    ) -> Result<()> {
        let h = ops.hash().c(d!())?;

        if self.coinbase.distribution_hist.contains(&h) {
            return Err(eg!("already exists"));
        }

        // Update fra distribution history first.
        self.coinbase.distribution_hist.insert(h);

        let mut v;
        for (k, am) in ops.data.allocation_table.into_iter() {
            v = self.coinbase.distribution_plan.entry(k).or_insert(0);
            *v = v.checked_add(am).ok_or(eg!("overflow"))?;
        }

        Ok(())
    }

    /// Do the final payment on staking structures.
    pub fn coinbase_pay(&mut self, tx: &Transaction) -> Result<()> {
        if !self.is_coinbase_ops(tx) {
            return Ok(());
        }

        if !self.seems_valid_coinbase_ops(tx) {
            return Err(eg!());
        }

        self.coinbase_collect_payments(tx)
            .c(d!())
            .map(|(distribution, delegation)| {
                self.coinbase_pay_fra_distribution(&distribution);
                self.coinbase_pay_delegation(&delegation);
            })
    }

    // Check if a tx contains any inputs from coinbase,
    // if it does, then it must pass all checkers about coinbase.
    #[inline(always)]
    fn is_coinbase_ops(&self, tx: &Transaction) -> bool {
        tx.body.operations.iter().any(|o| {
            if let Operation::TransferAsset(ref ops) = o {
                if ops
                    .body
                    .transfer
                    .inputs
                    .iter()
                    .any(|i| i.public_key == self.coinbase.pubkey)
                {
                    return true;
                }
            }
            false
        })
    }

    // Check if this is a valid coinbase operation.
    //
    // - only `TransferAsset` operations are allowed
    // - all inputs must be owned by `CoinBase`
    // - all inputs and outputs must be `NonConfidential`
    // - only FRA are involved in this transaction
    // - all outputs must be owned by addresses in 'fra distribution' or 'delegation'
    //
    // **NOTE:** amount is not checked in this function !
    fn seems_valid_coinbase_ops(&self, tx: &Transaction) -> bool {
        let inputs_is_valid = |o: &TransferAsset| {
            o.body
                .transfer
                .inputs
                .iter()
                .all(|i| i.public_key == self.coinbase.pubkey)
        };

        let outputs_is_valid = |o: &TransferAsset| {
            o.body.transfer.outputs.iter().all(|i| {
                self.addr_is_in_distribution_plan(&i.public_key)
                    || self.addr_is_in_frozen_delegation(&i.public_key)
            })
        };

        let only_nonconfidential_fra = |o: &TransferAsset| {
            o.body
                .transfer
                .inputs
                .iter()
                .chain(o.body.transfer.outputs.iter())
                .all(|i| {
                    if let XfrAssetType::NonConfidential(t) = i.asset_type {
                        if ASSET_TYPE_FRA == t {
                            return matches!(i.amount, XfrAmount::NonConfidential(_));
                        }
                    }
                    false
                })
        };

        let ops_is_valid = |ops: &Operation| {
            if let Operation::TransferAsset(o) = ops {
                inputs_is_valid(o) && outputs_is_valid(o) && only_nonconfidential_fra(o)
            } else {
                false
            }
        };

        tx.body.operations.iter().all(|o| ops_is_valid(o))
    }

    fn coinbase_collect_payments(
        &self,
        tx: &Transaction,
    ) -> Result<(HashMap<XfrPublicKey, u64>, HashMap<XfrPublicKey, u64>)> {
        let mut v: &mut u64;
        let mut distribution = map! {};
        let mut delegation = map! {};

        for o in tx.body.operations.iter() {
            if let Operation::TransferAsset(ref ops) = o {
                for u in ops.body.transfer.outputs.iter() {
                    if let XfrAssetType::NonConfidential(t) = u.asset_type {
                        if t == ASSET_TYPE_FRA {
                            if let XfrAmount::NonConfidential(am) = u.amount {
                                if self.addr_is_in_distribution_plan(&u.public_key) {
                                    v = distribution.entry(u.public_key).or_insert(0);
                                    *v = v.checked_add(am).ok_or(eg!("overflow"))?;
                                }
                                if self.addr_is_in_frozen_delegation(&u.public_key) {
                                    v = delegation.entry(u.public_key).or_insert(0);
                                    *v = v.checked_add(am).ok_or(eg!("overflow"))?;
                                }
                            }
                        }
                    }
                }
            }
        }

        let xa = distribution
            .iter()
            .any(|(addr, am)| self.coinbase.distribution_plan.get(addr).unwrap() != am);
        let xb = delegation.iter().any(|(addr, am)| {
            self.delegation_get(addr).unwrap().rwd_amount != *am as i64
        });

        if xa || xb {
            return Err(eg!("amount not match"));
        }

        Ok((distribution, delegation))
    }

    // amounts have been checked in `coinbase_collect_payments`,
    fn coinbase_pay_fra_distribution(&mut self, payments: &HashMap<XfrPublicKey, u64>) {
        self.coinbase
            .distribution_plan
            .iter_mut()
            .for_each(|(k, am)| {
                // once paid, it was all paid
                if payments.contains_key(k) {
                    *am = 0;
                }
            });

        // clean 'completely paid' item
        self.coinbase.distribution_plan =
            mem::take(&mut self.coinbase.distribution_plan)
                .into_iter()
                .filter(|(_, am)| 0 < *am)
                .collect();
    }

    // - amounts have been checked in `coinbase_collect_payments`
    // - pubkey existances have been checked in `seems_valid_coinbase_ops`
    // - delegation states has been checked in `addr_is_in_frozen_delegation`
    #[inline(always)]
    fn coinbase_pay_delegation(&mut self, payments: &HashMap<XfrPublicKey, u64>) {
        payments.keys().for_each(|k| {
            self.delegation_get_mut(k).unwrap().state = DelegationState::Paid;
        });
    }

    // For addresses in delegation state,
    // postpone the distribution until the delegation ends.
    #[inline(always)]
    fn addr_is_in_distribution_plan(&self, pk: &XfrPublicKey) -> bool {
        self.coinbase.distribution_plan.contains_key(pk)
            && !self.di.addr_map.contains_key(pk)
    }

    #[inline(always)]
    fn addr_is_in_frozen_delegation(&self, pk: &XfrPublicKey) -> bool {
        if let Some(dlg) = self.di.addr_map.get(pk) {
            matches!(dlg.state, DelegationState::Frozen)
        } else {
            false
        }
    }

    #[inline(always)]
    fn addr_is_validator(&self, pk: &XfrPublicKey) -> bool {
        self.validator_get_current()
            .map(|v| v.data.contains_key(pk))
            .unwrap_or(false)
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn fra_distribution_get_plan(&self) -> &BTreeMap<XfrPublicKey, u64> {
        &self.coinbase.distribution_plan
    }
}

const FRA: u64 = 10_u64.pow(FRA_DECIMALS as u32);
const MIN_DELEGATION_AMOUNT: u64 = 32 * FRA;
const MAX_DELEGATION_AMOUNT: u64 = 32_0000 * FRA;

/// The 24-words mnemonic of 'FRA CoinBase Address'.
pub const COIN_BASE_MNEMONIC: &str = "load second west source excuse skin thought inside wool kick power tail universe brush kid butter bomb other mistake oven raw armed tree walk";

// A limitation from
// [tendermint](https://docs.tendermint.com/v0.33/spec/abci/apps.html#validator-updates)
//
// > Note that the maximum total power of the validator set
// > is bounded by MaxTotalVotingPower = MaxInt64 / 8.
// > Applications are responsible for ensuring
// > they do not make changes to the validator set
// > that cause it to exceed this limit.
const MAX_TOTAL_POWER: i64 = i64::MAX / 8;

// The max vote power of any validator
// can not exceed 20% of total power.
const MAX_POWER_PERCENT_PER_VALIDATOR: [u128; 2] = [1, 5];

// Block time interval, in seconds.
const BLOCK_INTERVAL: u64 = 15;

/// The lock time after the delegation expires, about 21 days.
pub const FROZEN_BLOCK_CNT: u64 = 3600 * 24 * 21 / BLOCK_INTERVAL;

// used to express some descriptive information
type Memo = String;

// block height of tendermint
pub(crate) type BlockHeight = u64;

// use i64 to keep compatible with the logic of asset penalty
type Amount = i64;

// sha256(pubkey)[:20]
type TendermintAddr = String;
type TendermintAddrRef<'a> = &'a str;

type ValidatorInfo = BTreeMap<BlockHeight, ValidatorData>;

/// Data of the effective validators on a specified height.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatorData {
    pub(crate) height: BlockHeight,
    pub(crate) cosig_rule: CoSigRule,
    /// Major data of validators.
    pub data: BTreeMap<XfrPublicKey, Validator>,
    // <tendermint validator address> => XfrPublicKey
    addr_td_to_app: BTreeMap<TendermintAddr, XfrPublicKey>,
}

impl ValidatorData {
    #[allow(missing_docs)]
    pub fn new(h: BlockHeight, v_set: Vec<Validator>) -> Result<Self> {
        if h < 1 {
            return Err(eg!("invalid start height"));
        }

        let mut data = BTreeMap::new();
        let mut addr_td_to_app = BTreeMap::new();
        for v in v_set.into_iter() {
            addr_td_to_app.insert(td_pubkey_to_td_addr(&v.td_pubkey), v.id);
            if data.insert(v.id, v).is_some() {
                return Err(eg!("duplicate entries"));
            }
        }

        let cosig_rule = Self::gen_cosig_rule(data.keys().copied().collect()).c(d!())?;

        Ok(ValidatorData {
            height: h,
            cosig_rule,
            data,
            addr_td_to_app,
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
    pub fn get_validators(&self) -> &BTreeMap<XfrPublicKey, Validator> {
        &self.data
    }
}

// the same address is not allowed to delegate twice at the same time,
// so it is feasible to use `XfrPublicKey` as the map key.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
struct DelegationInfo {
    addr_map: BTreeMap<XfrPublicKey, Delegation>,
    end_height_map: BTreeMap<BlockHeight, BTreeSet<XfrPublicKey>>,
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

// All transactions sent from CoinBase must support idempotence.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CoinBase {
    pubkey: XfrPublicKey,
    keypair: XfrKeyPair,
    bank: BTreeSet<TxoSID>,
    distribution_hist: BTreeSet<Digest>,
    distribution_plan: BTreeMap<XfrPublicKey, u64>,
}

impl Eq for CoinBase {}

impl PartialEq for CoinBase {
    fn eq(&self, other: &Self) -> bool {
        self.pubkey == other.pubkey
    }
}

impl Default for CoinBase {
    fn default() -> Self {
        Self::gen()
    }
}

impl CoinBase {
    fn gen() -> Self {
        let keypair = pnk!(wallet::restore_keypair_from_mnemonic_default(
            COIN_BASE_MNEMONIC
        ));
        CoinBase {
            pubkey: keypair.get_pk(),
            keypair,
            bank: BTreeSet::new(),
            distribution_hist: BTreeSet::new(),
            distribution_plan: BTreeMap::new(),
        }
    }
}

// sha256(pubkey)[:20]
fn td_pubkey_to_td_addr(pubkey: &[u8]) -> String {
    hex::encode(&sha2::Sha256::digest(pubkey)[..20])
}

#[cfg(test)]
mod test {
    // TODO
}
