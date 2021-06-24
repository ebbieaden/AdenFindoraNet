mod client;
mod genesis;
mod keeper;
mod message;

use abci::*;
use keeper::Keeper;
pub use message::Message;
use primitives::{
    module::{AppModule, AppModuleBasic, AppModuleGenesis},
    transaction::TxMsg,
};
use ruc::*;

pub const MODULE_NAME: &str = "ethereum";

pub struct EthereumModule {
    name: String,
    keeper: Keeper,
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
    fn tx_route(&self, msg: Box<dyn TxMsg>) -> Result<()> {
        msg_handler(&self.keeper, msg)
    }

    fn query_route(&self, path: Vec<&str>, req: &RequestQuery) -> ResponseQuery {
        query_handler(path, req)
    }

    fn begin_block(&mut self, _req: RequestBeginBlock) {
        todo!()
    }

    fn end_block(&mut self, _req: RequestEndBlock) -> ResponseEndBlock {
        todo!()
    }
}

fn msg_handler(_k: &Keeper, msg: Box<dyn TxMsg>) -> Result<()> {
    msg.as_any()
        .downcast_ref::<Message>()
        .ok_or(eg!("invalid transaction message"))
        .and_then(|m| match m {
            Message::Transact(_tx) => Ok(()),
        })
}

fn query_handler(_path: Vec<&str>, _req: &RequestQuery) -> ResponseQuery {
    ResponseQuery::new()
}
