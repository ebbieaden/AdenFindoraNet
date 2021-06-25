mod client;
mod genesis;
mod keeper;
mod message;

use abci::*;
use keeper::{EvmRunner, Keeper};
pub use message::Action;
use primitives::{
    module::{AppModule, AppModuleBasic, AppModuleGenesis},
    transaction::Executable,
};
use ruc::*;

pub const MODULE_NAME: &str = "evm";

pub struct EvmModule {
    name: String,
    keeper: Keeper,
}

impl EvmModule {
    pub fn new() -> EvmModule {
        EvmModule {
            name: MODULE_NAME.to_string(),
            keeper: Keeper::new(),
        }
    }
}

impl AppModuleBasic for EvmModule {
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

impl AppModuleGenesis for EvmModule {
    fn init_genesis(&self) {
        todo!()
    }

    fn export_genesis(&self) {
        todo!()
    }
}

impl AppModule for EvmModule {
    // fn tx_route(&self, msg: Box<dyn Executable>) -> Result<()> {
    //     msg_handler(&self.keeper, msg)
    // }

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

// fn msg_handler(k: &Keeper, msg: Box<dyn Executable>) -> Result<()> {
//     msg.as_any()
//         .downcast_ref::<Message>()
//         .ok_or(eg!("invalid transaction message"))
//         .and_then(|m| match m {
//             Message::Call(params) => k.call(&params),
//             Message::Create(params) => k.create(&params),
//             Message::Create2(params) => k.create2(&params),
//         })
// }

fn query_handler(_path: Vec<&str>, _req: &RequestQuery) -> ResponseQuery {
    ResponseQuery::new()
}
