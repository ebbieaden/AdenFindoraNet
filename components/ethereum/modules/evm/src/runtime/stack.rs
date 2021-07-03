use crate::{AddressMapping, Config, OnChargeEVMTransaction};
use evm::backend::Backend as BackendT;
use evm::executor::{StackExecutor, StackState as StackStateT, StackSubstateMetadata};
use evm::{ExitError, ExitReason, Transfer};
use fp_core::macros::Get;
use fp_evm::{CallInfo, CreateInfo, ExecutionInfo, Log, Vicinity};
use primitive_types::{H160, H256, U256};
use sha3::{Digest, Keccak256};
use std::collections::btree_set::BTreeSet;
use std::marker::PhantomData;
use std::mem;

pub struct SubstrateStackSubstate<'config> {
    pub metadata: StackSubstateMetadata<'config>,
    pub deletes: BTreeSet<H160>,
    pub logs: Vec<Log>,
    pub parent: Option<Box<SubstrateStackSubstate<'config>>>,
}

impl<'config> SubstrateStackSubstate<'config> {
    pub fn metadata(&self) -> &StackSubstateMetadata<'config> {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
        &mut self.metadata
    }

    pub fn enter(&mut self, gas_limit: u64, is_static: bool) {
        let mut entering = Self {
            metadata: self.metadata.spit_child(gas_limit, is_static),
            parent: None,
            deletes: BTreeSet::new(),
            logs: Vec::new(),
        };
        mem::swap(&mut entering, self);

        self.parent = Some(Box::new(entering));

        // TODO
        // sp_io::storage::start_transaction();
    }

    pub fn exit_commit(&mut self) -> Result<(), ExitError> {
        let mut exited = *self.parent.take().expect("Cannot commit on root substate");
        mem::swap(&mut exited, self);

        self.metadata.swallow_commit(exited.metadata)?;
        self.logs.append(&mut exited.logs);
        self.deletes.append(&mut exited.deletes);

        // TODO
        // sp_io::storage::commit_transaction();
        Ok(())
    }

    pub fn exit_revert(&mut self) -> Result<(), ExitError> {
        let mut exited = *self.parent.take().expect("Cannot discard on root substate");
        mem::swap(&mut exited, self);
        self.metadata.swallow_revert(exited.metadata)?;

        // TODO
        // sp_io::storage::rollback_transaction();
        Ok(())
    }

    pub fn exit_discard(&mut self) -> Result<(), ExitError> {
        let mut exited = *self.parent.take().expect("Cannot discard on root substate");
        mem::swap(&mut exited, self);
        self.metadata.swallow_discard(exited.metadata)?;

        // TODO
        // sp_io::storage::rollback_transaction();
        Ok(())
    }

    pub fn deleted(&self, address: H160) -> bool {
        if self.deletes.contains(&address) {
            return true;
        }

        if let Some(parent) = self.parent.as_ref() {
            return parent.deleted(address);
        }

        false
    }

    pub fn set_deleted(&mut self, address: H160) {
        self.deletes.insert(address);
    }

    pub fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
        self.logs.push(Log {
            address,
            topics,
            data,
        });
    }
}

/// Substrate backend for EVM.
pub struct SubstrateStackState<'vicinity, 'config, T> {
    pub vicinity: &'vicinity Vicinity,
    pub substate: SubstrateStackSubstate<'config>,
    _marker: PhantomData<T>,
}

impl<'vicinity, 'config, T: Config> SubstrateStackState<'vicinity, 'config, T> {
    /// Create a new backend with given vicinity.
    pub fn new(
        vicinity: &'vicinity Vicinity,
        metadata: StackSubstateMetadata<'config>,
    ) -> Self {
        Self {
            vicinity,
            substate: SubstrateStackSubstate {
                metadata,
                deletes: BTreeSet::new(),
                logs: Vec::new(),
                parent: None,
            },
            _marker: PhantomData,
        }
    }
}

impl<'vicinity, 'config, T: Config> BackendT
    for SubstrateStackState<'vicinity, 'config, T>
{
    fn gas_price(&self) -> U256 {
        self.vicinity.gas_price
    }
    fn origin(&self) -> H160 {
        self.vicinity.origin
    }

    fn block_hash(&self, number: U256) -> H256 {
        // if number > U256::from(u32::max_value()) {
        //     H256::default()
        // } else {
        //     T::BlockHashMapping::block_hash(number.as_u32())
        // }
        todo!()
    }

    fn block_number(&self) -> U256 {
        // let number: u128 =
        //     frame_system::Module::<T>::block_number().unique_saturated_into();
        // U256::from(number)
        todo!()
    }

    fn block_coinbase(&self) -> H160 {
        // Module::<T>::find_author()
        todo!()
    }

    fn block_timestamp(&self) -> U256 {
        // let now: u128 = pallet_timestamp::Module::<T>::get().unique_saturated_into();
        // U256::from(now / 1000)
        todo!()
    }

    fn block_difficulty(&self) -> U256 {
        U256::zero()
    }

    fn block_gas_limit(&self) -> U256 {
        U256::zero()
    }

    fn chain_id(&self) -> U256 {
        U256::from(T::ChainId::get())
    }

    fn exists(&self, _address: H160) -> bool {
        true
    }

    fn basic(&self, address: H160) -> evm::backend::Basic {
        // let account = Module::<T>::account_basic(&address);
        //
        // evm::backend::Basic {
        //     balance: account.balance,
        //     nonce: account.nonce,
        // }
        todo!()
    }

    fn code(&self, address: H160) -> Vec<u8> {
        // AccountCodes::get(&address)
        todo!()
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        // AccountStorages::get(address, index)
        todo!()
    }

    fn original_storage(&self, _address: H160, _index: H256) -> Option<H256> {
        None
    }
}

impl<'vicinity, 'config, T: Config> StackStateT<'config>
    for SubstrateStackState<'vicinity, 'config, T>
{
    fn metadata(&self) -> &StackSubstateMetadata<'config> {
        self.substate.metadata()
    }

    fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
        self.substate.metadata_mut()
    }

    fn enter(&mut self, gas_limit: u64, is_static: bool) {
        self.substate.enter(gas_limit, is_static)
    }

    fn exit_commit(&mut self) -> Result<(), ExitError> {
        self.substate.exit_commit()
    }

    fn exit_revert(&mut self) -> Result<(), ExitError> {
        self.substate.exit_revert()
    }

    fn exit_discard(&mut self) -> Result<(), ExitError> {
        self.substate.exit_discard()
    }

    fn is_empty(&self, address: H160) -> bool {
        // Module::<T>::is_account_empty(&address)
        todo!()
    }

    fn deleted(&self, address: H160) -> bool {
        self.substate.deleted(address)
    }

    fn inc_nonce(&mut self, address: H160) {
        // let account_id = T::AddressMapping::into_account_id(address);
        // frame_system::Module::<T>::inc_account_nonce(&account_id);
        todo!()
    }

    fn set_storage(&mut self, address: H160, index: H256, value: H256) {
        // if value == H256::default() {
        //     log::debug!(
        //         target: "evm",
        //         "Removing storage for {:?} [index: {:?}]",
        //         address,
        //         index,
        //     );
        //     AccountStorages::remove(address, index);
        // } else {
        //     log::debug!(
        //         target: "evm",
        //         "Updating storage for {:?} [index: {:?}, value: {:?}]",
        //         address,
        //         index,
        //         value,
        //     );
        //     AccountStorages::insert(address, index, value);
        // }
        todo!()
    }

    fn reset_storage(&mut self, address: H160) {
        // AccountStorages::remove_prefix(address);
        todo!()
    }

    fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
        self.substate.log(address, topics, data)
    }

    fn set_deleted(&mut self, address: H160) {
        self.substate.set_deleted(address)
    }

    fn set_code(&mut self, address: H160, code: Vec<u8>) {
        log::debug!(
            target: "evm",
            "Inserting code ({} bytes) at {:?}",
            code.len(),
            address
        );
        // Module::<T>::create_account(address, code);
        todo!()
    }

    fn transfer(&mut self, transfer: Transfer) -> Result<(), ExitError> {
        // let source = T::AddressMapping::into_account_id(transfer.source);
        // let target = T::AddressMapping::into_account_id(transfer.target);
        //
        // T::Currency::transfer(
        //     &source,
        //     &target,
        //     transfer.value.low_u128().unique_saturated_into(),
        //     ExistenceRequirement::AllowDeath,
        // )
        // .map_err(|_| ExitError::OutOfFund)
        todo!()
    }

    fn reset_balance(&mut self, _address: H160) {
        // Do nothing on reset balance in Substrate.
        //
        // This function exists in EVM because a design issue
        // (arguably a bug) in SELFDESTRUCT that can cause total
        // issurance to be reduced. We do not need to replicate this.
    }

    fn touch(&mut self, _address: H160) {
        // Do nothing on touch in Substrate.
        //
        // EVM pallet considers all accounts to exist, and distinguish
        // only empty and non-empty accounts. This avoids many of the
        // subtle issues in EIP-161.
    }
}
