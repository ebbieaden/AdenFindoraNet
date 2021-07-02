mod app;
mod types;

use abci::Header;
use fp_core::{
    context::Context,
    crypto::Address,
    module::{AppModule, AppModuleBasic},
    parameter_types,
    transaction::{Applyable, Executable, ValidateUnsigned},
};
use parking_lot::RwLock;
use primitive_types::U256;
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use storage::{db::FinDB, state::ChainState};

pub use types::*;

const APP_NAME: &str = "findora";
const APP_DB_NAME: &str = "findora_db";

pub struct BaseApp {
    /// application name from abci.Info
    name: String,
    /// application's version string
    version: String,
    /// application's protocol version that increments on every upgrade
    /// if BaseApp is passed to the upgrade keeper's NewKeeper method.
    app_version: u64,
    /// Chain persistent state
    chain_state: Arc<RwLock<ChainState<FinDB>>>,
    /// volatile states
    ///
    /// check_state is set on InitChain and reset on Commit
    /// deliver_state is set on InitChain and BeginBlock and set to nil on Commit
    check_state: Context,
    deliver_state: Context,
    /// Ordered module set
    ethereum_module: module_ethereum::App<Self>,
    evm_module: module_evm::App<Self>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, Copy)]
pub enum RunTxMode {
    /// Check a transaction
    Check = 0,
    /// Recheck a (pending) transaction after a commit
    ReCheck = 1,
    /// Simulate a transaction
    Simulate = 2,
    /// Deliver a transaction
    Deliver = 3,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Ethereum(module_ethereum::Action),
    Evm(module_evm::Action),
}

parameter_types! {
    pub const ChainId: u64 = 42;
    pub BlockGasLimit: U256 = U256::from(u32::max_value());
}

impl module_ethereum::Config for BaseApp {}

impl module_evm::Config for BaseApp {
    type Runner = module_evm::App<Self>;
    type ChainId = ChainId;
    type BlockGasLimit = BlockGasLimit;
}

impl BaseApp {
    pub fn new(path: &Path) -> Result<Self> {
        let fdb = FinDB::open(path)?;
        let chain_state =
            Arc::new(RwLock::new(ChainState::new(fdb, APP_DB_NAME.to_string())));

        Ok(BaseApp {
            name: APP_NAME.to_string(),
            version: "1.0.0".to_string(),
            app_version: 1,
            chain_state: chain_state.clone(),
            check_state: Context::new(chain_state.clone()),
            deliver_state: Context::new(chain_state),
            ethereum_module: Default::default(),
            evm_module: module_evm::App::new(),
        })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn version(&self) -> String {
        self.version.clone()
    }

    pub fn app_version(&self) -> u64 {
        self.app_version
    }

    pub fn handle_query(
        &self,
        mut path: Vec<&str>,
        req: &abci::RequestQuery,
    ) -> abci::ResponseQuery {
        let mut resp = abci::ResponseQuery::new();
        if 0 == path.len() {
            resp.set_code(1);
            resp.set_log("Invalid custom query path without module route!".to_string());
            return resp;
        }
        let ctx = self.create_query_context(req.height, req.prove);
        if let Err(e) = ctx {
            resp.set_code(1);
            resp.set_log(format!("Cannot create query context with err: {}!", e));
            return resp;
        }

        let module_name = path.remove(0);
        if module_name == self.ethereum_module.name().as_str() {
            self.ethereum_module.query_route(ctx.unwrap(), path, req)
        } else if module_name == self.evm_module.name().as_str() {
            self.evm_module.query_route(ctx.unwrap(), path, req)
        } else {
            resp.set_code(1);
            resp.set_log(format!("Invalid query module route: {}!", module_name));
            resp
        }

        // if let Some(am) = self
        //     .modules
        //     .iter()
        //     .find(|&m| m.name().as_str() == module_name)
        // {
        //     am.query_route(path, req)
        // } else {
        //     resp.set_code(1);
        //     resp.set_log(format!("Invalid query module route: {}!", module_name));
        //     resp
        // }
    }

    pub fn handle_tx(
        &mut self,
        mode: RunTxMode,
        tx: UncheckedTransaction,
        tx_bytes: Vec<u8>,
    ) -> Result<()> {
        let checked = tx.clone().check()?;
        let ctx = self.retrieve_context(mode, tx_bytes);

        // add match field if tx is unsigned transaction
        match tx.function {
            Action::Ethereum(action) => Self::dispatch::<
                module_ethereum::Action,
                module_ethereum::App<BaseApp>,
            >(ctx.clone(), mode, action, checked),
            _ => Self::dispatch::<Action, BaseApp>(
                ctx.clone(),
                mode,
                tx.function,
                checked,
            ),
        }
    }
}

impl BaseApp {
    pub fn set_check_state(&mut self, header: Header) {
        self.check_state.header = header;
    }

    pub fn set_deliver_state(&mut self, header: Header) {
        self.deliver_state.header = header;
    }

    /// retrieve the context for the txBytes and other memoized values.
    pub fn retrieve_context(
        &mut self,
        mode: RunTxMode,
        tx_bytes: Vec<u8>,
    ) -> &mut Context {
        let ctx = if mode == RunTxMode::Deliver {
            &mut self.deliver_state
        } else {
            &mut self.check_state
        };
        ctx.tx = tx_bytes;

        if mode == RunTxMode::ReCheck {
            ctx.recheck_tx = true;
        }
        ctx
    }

    pub fn create_query_context(&self, mut height: i64, prove: bool) -> Result<Context> {
        if height < 0 {
            return Err(eg!(
                "cannot query with height < 0; please provide a valid height"
            ));
        }
        // when a client did not provide a query height, manually inject the latest
        if height == 0 {
            height = self.chain_state.read().height()? as i64;
        }
        if height <= 1 && prove {
            return Err(eg!(
                "cannot query with proof when height <= 1; please provide a valid height"
            ));
        }

        Ok(Context::new(self.chain_state.clone()))
    }
}

impl BaseApp {
    fn dispatch<Call, Module>(
        ctx: Context,
        mode: RunTxMode,
        action: Call,
        tx: CheckedTransaction,
    ) -> Result<()>
    where
        Module: ValidateUnsigned<Call = Call>,
        Module: Executable<Origin = Address, Call = Call>,
    {
        // TODO gas check、get ctx.store

        let origin_tx = convert_unsigned_transaction::<Call>(action, tx);

        origin_tx.validate::<Module>(ctx.clone())?;

        if mode == RunTxMode::Deliver {
            origin_tx.apply::<Module>(ctx)?;
        }
        Ok(())
    }
}

impl ValidateUnsigned for BaseApp {
    type Call = Action;

    fn pre_execute(call: &Self::Call, _ctx: Context) -> Result<()> {
        #[allow(unreachable_patterns)]
        match call {
            _ => Ok(()),
        }
    }

    fn validate_unsigned(call: &Self::Call, _ctx: Context) -> Result<()> {
        #[allow(unreachable_patterns)]
        match call {
            _ => Err(eg!(
                "Could not find an unsigned validator for the unsigned transaction"
            )),
        }
    }
}

impl Executable for BaseApp {
    type Origin = Address;
    type Call = Action;

    fn execute(
        origin: Option<Self::Origin>,
        call: Self::Call,
        ctx: Context,
    ) -> Result<()> {
        match call {
            Action::Ethereum(action) => {
                module_ethereum::App::<Self>::execute(origin, action, ctx)
            }
            Action::Evm(action) => module_evm::App::<Self>::execute(origin, action, ctx),
        }
    }
}
