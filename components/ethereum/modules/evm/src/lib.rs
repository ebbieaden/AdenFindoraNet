mod basic;
mod client;
mod genesis;
pub mod runtime;
mod storage;

use abci::{RequestEndBlock, RequestQuery, ResponseEndBlock, ResponseQuery};
use evm::Config as EvmConfig;
use fp_core::{
    context::Context, crypto::Address, macros::Get, module::AppModule,
    transaction::Executable,
};
use fp_evm::{Account, CallInfo, CreateInfo, PrecompileSet};
use primitive_types::{H160, U256};
use ruc::Result;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub use runtime::*;

static ISTANBUL_CONFIG: EvmConfig = EvmConfig::istanbul();

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

pub trait Config: Sized {
    /// Mapping from address to account id.
    type AddressMapping: AddressMapping;
    /// The block gas limit. Can be a simple constant, or an adjustment algorithm in another pallet.
    type BlockGasLimit: Get<U256>;
    /// Chain ID of EVM.
    type ChainId: Get<u64>;
    /// Calculator for current gas price.
    type FeeCalculator: FeeCalculator;
    /// To handle fee deduction for EVM transactions.
    type OnChargeTransaction: OnChargeEVMTransaction<Self>;
    /// Precompiles associated with this EVM engine.
    type Precompiles: PrecompileSet;
    /// EVM execution runner.
    type Runner: Runner;
    /// EVM config used in the module.
    fn config() -> &'static EvmConfig {
        &ISTANBUL_CONFIG
    }
}

/// Trait that outputs the current transaction gas price.
pub trait FeeCalculator {
    /// Return the minimal required gas price.
    fn min_gas_price() -> U256;
}

impl FeeCalculator for () {
    fn min_gas_price() -> U256 {
        U256::zero()
    }
}

pub trait AddressMapping {
    fn into_account_id(address: H160) -> Address;
}

/// Ethereum address mapping.
pub struct EthereumAddressMapping;

impl AddressMapping for EthereumAddressMapping {
    fn into_account_id(address: H160) -> Address {
        todo!()
    }
}

/// Handle withdrawing, refunding and depositing of transaction fees.
/// Similar to `OnChargeTransaction` of `pallet_transaction_payment`
pub trait OnChargeEVMTransaction<T: Config> {
    type LiquidityInfo: Default;

    /// Before the transaction is executed the payment of the transaction fees
    /// need to be secured.
    fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo>;

    /// After the transaction was executed the actual fee can be calculated.
    /// This function should refund any overpaid fees and optionally deposit
    /// the corrected amount.
    fn correct_and_deposit_fee(
        who: &H160,
        corrected_fee: U256,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<()>;
}

/// Implements the transaction payment for a module implementing the `Currency`
/// trait (eg. the pallet_balances) using an unbalance handler (implementing
/// `OnUnbalanced`).
pub struct EVMCurrencyAdapter;

impl<C: Config> OnChargeEVMTransaction<C> for EVMCurrencyAdapter {
    type LiquidityInfo = ();

    fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo> {
        todo!()
    }

    fn correct_and_deposit_fee(
        who: &H160,
        corrected_fee: U256,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<()> {
        todo!()
    }
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            name: "evm".to_string(),
            phantom: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Call(Call),
    Create(Create),
    Create2(Create2),
}

impl<C: Config> AppModule for App<C> {
    fn query_route(
        &self,
        _ctx: Context,
        _path: Vec<&str>,
        _req: &RequestQuery,
    ) -> ResponseQuery {
        todo!()
    }

    fn end_block(
        &mut self,
        _ctx: &mut Context,
        _req: &RequestEndBlock,
    ) -> ResponseEndBlock {
        todo!()
    }
}

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        _origin: Option<Self::Origin>,
        _call: Self::Call,
        _ctx: Context,
    ) -> Result<()> {
        todo!()
    }
}

impl<C: Config> App<C> {
    /// Remove an account.
    pub fn remove_account(_address: &H160) {
        // if AccountCodes::contains_key(address) {
        //     let account_id = T::AddressMapping::into_account_id(*address);
        //     let _ = frame_system::Module::<T>::dec_consumers(&account_id);
        // }
        //
        // AccountCodes::remove(address);
        // AccountStorages::remove_prefix(address);
    }

    /// Get the account basic in EVM format.
    pub fn account_basic(address: &H160) -> Account {
        let _account_id = C::AddressMapping::into_account_id(*address);

        // let nonce = frame_system::Module::<T>::account_nonce(&account_id);
        // let balance = T::Currency::free_balance(&account_id);
        //
        // Account {
        //     nonce: U256::from(UniqueSaturatedInto::<u128>::unique_saturated_into(nonce)),
        //     balance: U256::from(UniqueSaturatedInto::<u128>::unique_saturated_into(balance)),
        // }
        todo!()
    }
}
