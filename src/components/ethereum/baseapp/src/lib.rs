mod app;
mod types;

use app_ethereum::EthereumModule;
use app_evm::EvmModule;
use parking_lot::RwLock;
use primitives::{
    context::{CacheState, CommitState, CommitStore},
    crypto::Address,
    module::AppModule,
    transaction::{Applyable, Executable, ValidateUnsigned},
};
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use storage::{db::FinDB, state::ChainState};

use abci::Header;
use primitive_types::U256;
use primitives::context::Context;
use primitives::module::AppModuleBasic;
use primitives::parameter_types;
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
    commit_store: CommitStore,
    /// volatile states
    ///
    /// check_state is set on InitChain and reset on Commit
    /// deliver_state is set on InitChain and BeginBlock and set to nil on Commit
    check_state: CacheState,
    deliver_state: CommitState,
    /// Ordered module set
    ethereum_module: EthereumModule<Self>,
    evm_module: EvmModule<Self>,
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
    Ethereum(app_ethereum::Action),
    Evm(app_evm::Action),
}

impl Executable for Action {
    type Origin = Address;

    fn execute(self, origin: Option<Self::Origin>, ctx: Context) -> Result<()> {
        match self {
            Action::Ethereum(action) => action.execute(origin, ctx),
            Action::Evm(action) => action.execute(origin, ctx),
        }
    }
}

parameter_types! {
    pub const ChainId: u64 = 42;
    pub BlockGasLimit: U256 = U256::from(u32::max_value());
}

impl app_ethereum::Config for BaseApp {}

impl app_evm::Config for BaseApp {
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
            commit_store: CommitStore::new(chain_state.clone()),
            check_state: CacheState::new(chain_state.clone()),
            deliver_state: CommitState::new(chain_state),
            ethereum_module: Default::default(),
            evm_module: EvmModule::new(),
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

        let module_name = path.remove(0);
        if module_name == self.ethereum_module.name().as_str() {
            self.ethereum_module.query_route(path, req)
        } else if module_name == self.evm_module.name().as_str() {
            self.evm_module.query_route(path, req)
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
                app_ethereum::Action,
                EthereumModule<BaseApp>,
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
        self.check_state.ctx.header = header;
    }

    pub fn set_deliver_state(&mut self, header: Header) {
        self.deliver_state.ctx.header = header;
    }

    /// retrieve the context for the txBytes and other memoized values.
    pub fn retrieve_context(
        &mut self,
        mode: RunTxMode,
        tx_bytes: Vec<u8>,
    ) -> &mut Context {
        let ctx = if mode == RunTxMode::Deliver {
            &mut self.deliver_state.ctx
        } else {
            &mut self.check_state.ctx
        };
        ctx.tx = tx_bytes;

        if mode == RunTxMode::ReCheck {
            ctx.recheck_tx = true;
        }
        ctx
    }
}

impl BaseApp {
    fn dispatch<
        Call: Executable<Origin = Address>,
        Module: ValidateUnsigned<Call = Call>,
    >(
        ctx: Context,
        mode: RunTxMode,
        action: Call,
        tx: CheckedTransaction,
    ) -> Result<()> {
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
