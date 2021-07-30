mod precompile;

use ethereum_types::{Bloom, H160, H256, U256};
use evm::ExitReason;
use fp_core::context::Context;
use ruc::*;
use serde::{Deserialize, Serialize};

pub use evm::backend::{Basic as Account, Log};
pub use precompile::{LinearCostPrecompile, Precompile, PrecompileSet};

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize, Default)]
/// External input from the transaction.
pub struct Vicinity {
    /// Current transaction gas price.
    pub gas_price: U256,
    /// Origin of the transaction.
    pub origin: H160,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct ExecutionInfo<T> {
    pub exit_reason: ExitReason,
    pub value: T,
    pub used_gas: U256,
    pub logs: Vec<Log>,
}

pub type CallInfo = ExecutionInfo<Vec<u8>>;
pub type CreateInfo = ExecutionInfo<H160>;

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum CallOrCreateInfo {
    Call(CallInfo),
    Create(CreateInfo),
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct TransactionStatus {
    pub transaction_hash: H256,
    pub transaction_index: u32,
    pub from: H160,
    pub to: Option<H160>,
    pub contract_address: Option<H160>,
    pub logs: Vec<Log>,
    pub logs_bloom: Bloom,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        TransactionStatus {
            transaction_hash: H256::default(),
            transaction_index: 0 as u32,
            from: H160::default(),
            to: None,
            contract_address: None,
            logs: Vec::new(),
            logs_bloom: Bloom::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    pub source: H160,
    pub target: H160,
    pub input: Vec<u8>,
    pub value: U256,
    pub gas_limit: u64,
    pub gas_price: Option<U256>,
    pub nonce: Option<U256>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Create {
    pub source: H160,
    pub init: Vec<u8>,
    pub value: U256,
    pub gas_limit: u64,
    pub gas_price: Option<U256>,
    pub nonce: Option<U256>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Create2 {
    pub source: H160,
    pub init: Vec<u8>,
    pub salt: H256,
    pub value: U256,
    pub gas_limit: u64,
    pub gas_price: Option<U256>,
    pub nonce: Option<U256>,
}

pub trait Runner {
    fn call(ctx: &Context, args: Call, config: &evm::Config) -> Result<CallInfo>;

    fn create(ctx: &Context, args: Create, config: &evm::Config) -> Result<CreateInfo>;

    fn create2(ctx: &Context, args: Create2, config: &evm::Config)
    -> Result<CreateInfo>;
}
