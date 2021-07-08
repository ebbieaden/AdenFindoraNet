use crate::context::Context;
use ruc::Result;

/// AppModuleBasic is the standard form for basic non-dependant elements of an application module.
pub trait AppModuleBasic {
    /// Returns the module's name.
    fn name(&self) -> String;

    /// Returns default genesis state as raw bytes for the module.
    fn default_genesis(&self) -> Vec<u8>;

    /// Performs genesis state validation for the module.
    fn validate_genesis(&self) -> Result<()>;

    /// Registers the REST routes for the module.
    fn register_rest_routes(&self);

    /// Registers the gRPC Gateway routes for the module.
    fn register_grpc_gateway_routes(&self);

    /// Returns the root tx command for the module.
    fn get_tx_cmd(&self);

    /// Returns no root query command for the module.
    fn get_query_cmd(&self);
}

/// AppModuleGenesis is the standard form for an application module genesis functions
pub trait AppModuleGenesis {
    /// Performs genesis initialization for the module. It returns no validator updates.
    fn init_genesis(&self);

    /// Returns the exported genesis state as raw bytes for the module.
    fn export_genesis(&self);
}

/// AppModule is the standard form for an application module
pub trait AppModule: AppModuleBasic + AppModuleGenesis {
    /// query_route returns the application module's query response.
    fn query_route(
        &self,
        _ctx: Context,
        _path: Vec<&str>,
        _req: &abci::RequestQuery,
    ) -> abci::ResponseQuery {
        abci::ResponseQuery::new()
    }

    /// Tendermint consensus connection: called at the start of processing a block of transactions.
    fn begin_block(&mut self, _ctx: &mut Context, _req: &abci::RequestBeginBlock) {}

    /// Tendermint consensus connection: called at the end of the block.
    fn end_block(
        &mut self,
        _ctx: &mut Context,
        _req: &abci::RequestEndBlock,
    ) -> abci::ResponseEndBlock {
        abci::ResponseEndBlock::new()
    }
}
