use ethereum_types::{H160, H256, U256};
use primitives::transaction::TxMsg;
use ruc::Result;

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

impl TxMsg for Message {
    fn route_path(&self) -> String {
        crate::MODULE_NAME.to_string()
    }

    fn execute(&self) -> Result<()> {
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
