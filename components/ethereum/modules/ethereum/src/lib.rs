mod client;
mod genesis;
mod keeper;

use abci::*;
use keeper::Keeper;
use primitives::transaction::ValidateUnsigned;
use primitives::{crypto::Address32, module::*, transaction::Executable};
use ruc::Result;
use serde::{Deserialize, Serialize};

pub const MODULE_NAME: &str = "ethereum";

pub struct EthereumModule {
    name: String,
    keeper: Keeper,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transact(ethereum::Transaction),
}

impl Executable for Action {
    type Origin = Address32;

    fn execute(self, _origin: Option<Self::Origin>) -> Result<()> {
        match self {
            Action::Transact(tx) => Ok(()),
        }
    }
}

impl EthereumModule {
    pub fn new() -> EthereumModule {
        EthereumModule {
            name: MODULE_NAME.to_string(),
            keeper: Keeper::new(),
        }
    }
}

impl AppModuleBasic for EthereumModule {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn default_genesis(&self) -> Vec<u8> {
        todo!()
    }

    fn validate_genesis(&self) -> Result<()> {
        todo!()
    }

    fn register_rest_routes(&self) {
        todo!()
    }

    fn register_grpc_gateway_routes(&self) {
        todo!()
    }

    fn get_tx_cmd(&self) {
        todo!()
    }

    fn get_query_cmd(&self) {
        todo!()
    }
}

impl AppModuleGenesis for EthereumModule {
    fn init_genesis(&self) {
        todo!()
    }

    fn export_genesis(&self) {
        todo!()
    }
}

impl AppModule for EthereumModule {
    fn query_route(&self, path: Vec<&str>, req: &RequestQuery) -> ResponseQuery {
        ResponseQuery::new()
    }

    fn begin_block(&mut self, _req: &RequestBeginBlock) {
        todo!()
    }

    fn end_block(&mut self, _req: &RequestEndBlock) -> ResponseEndBlock {
        todo!()
    }
}

impl ValidateUnsigned for EthereumModule {
    type Call = Action;

    fn validate_unsigned(call: &Self::Call) -> Result<()> {
        todo!()
    }
}
