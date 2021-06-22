use ethereum_types::{H160, H256, U256};
use ruc::Result;

pub const EVM_MODULE_NAME: &str = "evm";

pub struct Call {
    source: H160,
    target: H160,
    input: Vec<u8>,
    value: U256,
    gas_limit: u64,
    gas_price: Option<U256>,
    nonce: Option<U256>,
}

pub struct Create {
    source: H160,
    init: Vec<u8>,
    value: U256,
    gas_limit: u64,
    gas_price: Option<U256>,
    nonce: Option<U256>,
}

pub struct Create2 {
    source: H160,
    init: Vec<u8>,
    salt: H256,
    value: U256,
    gas_limit: u64,
    gas_price: Option<U256>,
    nonce: Option<U256>,
}

pub enum Message {
    Call(Call),
    Create(Create),
    Create2(Create2),
}

impl super::TxMsg for Message {
    fn route_path(&self) -> String {
        EVM_MODULE_NAME.to_string()
    }

    fn validate_basic(&self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
