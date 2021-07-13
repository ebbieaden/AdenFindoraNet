pub mod runner;
mod stack;

use ethereum_types::{H160, H256, U256};
use fp_core::context::Context;
use fp_evm::{CallInfo, CreateInfo};
use ruc::Result;
use serde::{Deserialize, Serialize};

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
