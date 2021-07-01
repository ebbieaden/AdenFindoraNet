use ethereum_types::{H160, H256, U256};
use fp_core::{context::Context, crypto::Address, transaction::Executable};
use ruc::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    source: H160,
    target: H160,
    input: Vec<u8>,
    value: U256,
    gas_limit: u64,
    gas_price: Option<U256>,
    nonce: Option<U256>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Create {
    source: H160,
    init: Vec<u8>,
    value: U256,
    gas_limit: u64,
    gas_price: Option<U256>,
    nonce: Option<U256>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Create2 {
    source: H160,
    init: Vec<u8>,
    salt: H256,
    value: U256,
    gas_limit: u64,
    gas_price: Option<U256>,
    nonce: Option<U256>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Call(Call),
    Create(Create),
    Create2(Create2),
}

pub trait Runner {
    fn call(_args: Call) -> Result<()>;

    fn create(_args: Create) -> Result<()>;

    fn create2(_args: Create2) -> Result<()>;
}
