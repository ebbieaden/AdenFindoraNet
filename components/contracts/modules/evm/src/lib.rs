mod basic;
mod client;
mod genesis;
pub mod impls;
pub mod runtime;

use evm::Config as EvmConfig;
use fp_core::{
    context::Context,
    crypto::Address,
    macros::Get,
    module::AppModule,
    transaction::{ActionResult, Executable},
};
use fp_evm::PrecompileSet;
use fp_traits::{
    account::AccountAsset,
    evm::{
        AddressMapping, BlockHashMapping, DecimalsMapping, FeeCalculator,
        OnChargeEVMTransaction,
    },
};
use primitive_types::U256;
use ruc::Result;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub use runtime::*;

static ISTANBUL_CONFIG: EvmConfig = EvmConfig::istanbul();

pub trait Config {
    type AccountAsset: AccountAsset<Address>;
    /// Mapping from address to account id.
    type AddressMapping: AddressMapping;
    /// The block gas limit. Can be a simple constant, or an adjustment algorithm in another pallet.
    type BlockGasLimit: Get<U256>;
    /// Block number to block hash.
    type BlockHashMapping: BlockHashMapping;
    /// Chain ID of EVM.
    type ChainId: Get<u64>;
    /// Mapping from eth decimals to native token decimals.
    type DecimalsMapping: DecimalsMapping;
    /// Calculator for current gas price.
    type FeeCalculator: FeeCalculator;
    /// To handle fee deduction for EVM transactions.
    type OnChargeTransaction: OnChargeEVMTransaction;
    /// Precompiles associated with this EVM engine.
    type Precompiles: PrecompileSet;
    /// EVM execution runner.
    type Runner: Runner;
    /// EVM config used in the module.
    fn config() -> &'static EvmConfig {
        &ISTANBUL_CONFIG
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Call(Call),
    Create(Create),
    Create2(Create2),
}

pub mod storage {
    use fp_storage::*;
    use primitive_types::{H160, H256};

    // The code corresponding to the contract account.
    generate_storage!(EVM, AccountCodes => Map<H160, Vec<u8>>);
    // Storage root hash related to the contract account.
    generate_storage!(EVM, AccountStorages => DoubleMap<H160, H256, H256>);
}

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            name: "evm".to_string(),
            phantom: Default::default(),
        }
    }
}

impl<C: Config> Default for App<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Config> AppModule for App<C> {}

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        _origin: Option<Self::Origin>,
        _call: Self::Call,
        _ctx: &Context,
    ) -> Result<ActionResult> {
        todo!()
    }
}
