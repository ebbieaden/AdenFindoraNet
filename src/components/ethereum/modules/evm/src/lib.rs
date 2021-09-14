mod client;
mod genesis;
mod message;

use abci::*;
use fp_core::{
    context::Context,
    crypto::Address,
    module::{AppModule, AppModuleBasic, AppModuleGenesis},
    support::*,
    transaction::Executable,
};
use primitive_types::U256;
use ruc::*;
use std::marker::PhantomData;

pub use message::*;

pub const MODULE_NAME: &str = "evm";

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

pub trait Config: Send + Sync {
    /// EVM execution runner.
    type Runner: Runner;
    /// Chain ID of EVM.
    type ChainId: Get<u64>;
    /// The block gas limit. Can be a simple constant, or an adjustment algorithm in another pallet.
    type BlockGasLimit: Get<U256>;
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            name: MODULE_NAME.to_string(),
            phantom: Default::default(),
        }
    }
}

impl<C: Config> AppModuleBasic for App<C> {
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

impl<C: Config> AppModuleGenesis for App<C> {
    fn init_genesis(&self) {
        todo!()
    }

    fn export_genesis(&self) {
        todo!()
    }
}

impl<C: Config> AppModule for App<C> {
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

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        _origin: Option<Self::Origin>,
        _call: Self::Call,
        _ctx: Context,
    ) -> Result<()> {
        todo!()
    }
}

impl<C: Config> Runner for App<C> {
    fn call(_args: Call) -> Result<()> {
        todo!()
    }

    fn create(_args: Create) -> Result<()> {
        todo!()
    }

    fn create2(_args: Create2) -> Result<()> {
        todo!()
    }
}

fn query_handler(_path: Vec<&str>, _req: &RequestQuery) -> ResponseQuery {
    ResponseQuery::new()
}
