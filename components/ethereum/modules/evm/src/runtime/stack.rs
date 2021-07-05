use crate::{storage::*, AddressMapping, App, Config};
use evm::{
    backend::Backend,
    executor::{StackState, StackSubstateMetadata},
    ExitError, Transfer,
};
use fp_core::{context::Context, macros::Get};
use fp_evm::{Log, Vicinity};
use primitive_types::{H160, H256, U256};
use std::{collections::btree_set::BTreeSet, marker::PhantomData, mem};

pub struct FindoraStackSubstate<'config> {
    pub metadata: StackSubstateMetadata<'config>,
    pub deletes: BTreeSet<H160>,
    pub logs: Vec<Log>,
    pub parent: Option<Box<FindoraStackSubstate<'config>>>,
}

impl<'config> FindoraStackSubstate<'config> {
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

/// Findora backend for EVM.
pub struct FindoraStackState<'context, 'vicinity, 'config, T> {
    pub ctx: &'context Context,
    pub vicinity: &'vicinity Vicinity,
    pub substate: FindoraStackSubstate<'config>,
    _marker: PhantomData<T>,
}

impl<'context, 'vicinity, 'config, T: Config>
    FindoraStackState<'context, 'vicinity, 'config, T>
{
    /// Create a new backend with given vicinity.
    pub fn new(
        ctx: &'context Context,
        vicinity: &'vicinity Vicinity,
        metadata: StackSubstateMetadata<'config>,
    ) -> Self {
        Self {
            ctx,
            vicinity,
            substate: FindoraStackSubstate {
                metadata,
                deletes: BTreeSet::new(),
                logs: Vec::new(),
                parent: None,
            },
            _marker: PhantomData,
        }
    }
}

impl<'context, 'vicinity, 'config, C: Config> Backend
    for FindoraStackState<'context, 'vicinity, 'config, C>
{
    fn gas_price(&self) -> U256 {
        self.vicinity.gas_price
    }
    fn origin(&self) -> H160 {
        self.vicinity.origin
    }

    fn block_hash(&self, _number: U256) -> H256 {
        // if number > U256::from(u32::max_value()) {
        //     H256::default()
        // } else {
        //     T::BlockHashMapping::block_hash(number.as_u32())
        // }
        todo!()
    }

    fn block_number(&self) -> U256 {
        let number = self.ctx.block_height();
        U256::from(number)
    }

    fn block_coinbase(&self) -> H160 {
        App::<C>::find_proposer(self.ctx)
    }

    fn block_timestamp(&self) -> U256 {
        U256::from(self.ctx.block_time().get_nanos())
    }

    fn block_difficulty(&self) -> U256 {
        U256::zero()
    }

    fn block_gas_limit(&self) -> U256 {
        U256::zero()
    }

    fn chain_id(&self) -> U256 {
        U256::from(C::ChainId::get())
    }

    fn exists(&self, _address: H160) -> bool {
        true
    }

    fn basic(&self, address: H160) -> evm::backend::Basic {
        let account = App::<C>::account_basic(&address);

        evm::backend::Basic {
            balance: account.balance,
            nonce: account.nonce,
        }
    }

    fn code(&self, address: H160) -> Vec<u8> {
        AccountCodes::get(self.ctx.store.clone(), &address).unwrap_or_default()
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        AccountStorages::get(self.ctx.store.clone(), &address, &index)
            .unwrap_or_default()
    }

    fn original_storage(&self, _address: H160, _index: H256) -> Option<H256> {
        None
    }
}

impl<'context, 'vicinity, 'config, C: Config> StackState<'config>
    for FindoraStackState<'context, 'vicinity, 'config, C>
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
        App::<C>::is_account_empty(self.ctx, &address)
    }

    fn deleted(&self, address: H160) -> bool {
        self.substate.deleted(address)
    }

    fn inc_nonce(&mut self, address: H160) {
        let _account_id = C::AddressMapping::into_account_id(address);
        // frame_system::Module::<T>::inc_account_nonce(&account_id);
        todo!()
    }

    fn set_storage(&mut self, address: H160, index: H256, value: H256) {
        if value == H256::default() {
            log::debug!(
                target: "evm",
                "Removing storage for {:?} [index: {:?}]",
                address,
                index,
            );
            AccountStorages::remove(self.ctx.store.clone(), &address, &index);
        } else {
            log::debug!(
                target: "evm",
                "Updating storage for {:?} [index: {:?}, value: {:?}]",
                address,
                index,
                value,
            );
            AccountStorages::insert(self.ctx.store.clone(), &address, &index, &value);
        }
    }

    fn reset_storage(&mut self, address: H160) {
        AccountStorages::remove_prefix(self.ctx.store.clone(), &address);
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
        App::<C>::create_account(self.ctx, address, code);
    }

    fn transfer(&mut self, transfer: Transfer) -> Result<(), ExitError> {
        // TODO target must bind fra address
        let _source = C::AddressMapping::into_account_id(transfer.source);
        let _target = C::AddressMapping::into_account_id(transfer.target);
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
