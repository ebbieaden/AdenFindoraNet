mod app;
mod modules;
mod types;

use crate::modules::ModuleManager;
use fp_core::{
    context::Context,
    crypto::Address,
    parameter_types,
    transaction::{Executable, ValidateUnsigned},
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
    modules: ModuleManager,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Ethereum(module_ethereum::Action),
    Evm(module_evm::Action),
    Account(module_account::Action),
}

impl module_account::Config for BaseApp {}

impl module_ethereum::Config for BaseApp {}

parameter_types! {
    pub const ChainId: u64 = 42;
    pub BlockGasLimit: U256 = U256::from(u32::max_value());
}

impl module_evm::Config for BaseApp {
    type AddressMapping = module_evm::impls::EthereumAddressMapping;
    type BlockGasLimit = BlockGasLimit;
    type ChainId = ChainId;
    type FeeCalculator = ();
    type OnChargeTransaction = module_evm::App<Self>;
    type Precompiles = ();
    type Runner = module_evm::runtime::runner::ActionRunner<Self>;
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
            modules: Default::default(),
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
            Action::Account(action) => {
                module_account::App::<Self>::execute(origin, action, ctx)
            }
        }
    }
}

impl BaseApp {
    fn create_query_context(&self, mut height: i64, prove: bool) -> Result<Context> {
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

    /// retrieve the context for the txBytes and other memoized values.
    fn retrieve_context(&mut self, mode: RunTxMode, tx_bytes: Vec<u8>) -> &mut Context {
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
}
