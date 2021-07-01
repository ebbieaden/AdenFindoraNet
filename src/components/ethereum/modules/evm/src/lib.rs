mod client;
mod genesis;
mod keeper;
mod message;

use abci::*;
use keeper::{EvmRunner, Keeper};
pub use message::Action;
use primitive_types::U256;
use primitives::{
    module::{AppModule, AppModuleBasic, AppModuleGenesis},
    support::*,
};
use ruc::*;
use std::marker::PhantomData;

pub const MODULE_NAME: &str = "evm";

pub struct EvmModule<C> {
    name: String,
    keeper: Keeper,
    phantom: PhantomData<C>,
}

pub trait Config: Send + Sync {
    // /// EVM execution runner.
    // type Runner: EvmRunner;
    /// Chain ID of EVM.
    type ChainId: Get<u64>;
    /// The block gas limit. Can be a simple constant, or an adjustment algorithm in another pallet.
    type BlockGasLimit: Get<U256>;
}

impl<C: Config> EvmModule<C> {
    pub fn new() -> Self {
        EvmModule {
            name: MODULE_NAME.to_string(),
            keeper: Keeper::new(),
            phantom: Default::default(),
        }
    }
}

impl<C: Config> AppModuleBasic for EvmModule<C> {
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

impl<C: Config> AppModuleGenesis for EvmModule<C> {
    fn init_genesis(&self) {
        todo!()
    }

    fn export_genesis(&self) {
        todo!()
    }
}

impl<C: Config> AppModule for EvmModule<C> {
    fn query_route(&self, path: Vec<&str>, req: &RequestQuery) -> ResponseQuery {
        query_handler(path, req)
    }

    fn begin_block(&mut self, _req: &RequestBeginBlock) {
        todo!()
    }

    fn end_block(&mut self, _req: &RequestEndBlock) -> ResponseEndBlock {
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
