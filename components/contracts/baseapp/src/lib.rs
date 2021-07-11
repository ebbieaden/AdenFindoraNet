mod app;
mod modules;
mod types;

use crate::modules::ModuleManager;
use abci::Header;
use fp_core::account::SmartAccount;
use fp_core::{
    context::Context,
    crypto::Address,
    ensure, parameter_types,
    transaction::{Executable, ValidateUnsigned},
};
use fp_traits::account::AccountAsset;
use ledger::data_model::Transaction as FindoraTransaction;
use parking_lot::RwLock;
use primitive_types::U256;
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use storage::{db::FinDB, state::ChainState};
use zei::xfr::sig::XfrPublicKey;

pub use types::*;

const APP_NAME: &str = "findora";
const APP_DB_NAME: &str = "findora_db";

pub struct BaseApp {
    /// application name from abci.Info
    pub name: String,
    /// application's version string
    pub version: String,
    /// application's protocol version that increments on every upgrade
    /// if BaseApp is passed to the upgrade keeper's NewKeeper method.
    pub app_version: u64,
    /// Chain persistent state
    pub chain_state: Arc<RwLock<ChainState<FinDB>>>,
    /// volatile states
    ///
    /// check_state is set on InitChain and reset on Commit
    /// deliver_state is set on InitChain and BeginBlock and set to nil on Commit
    pub check_state: Context,
    pub deliver_state: Context,
    /// Ordered module set
    pub modules: ModuleManager,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Account(module_account::Action),
    Ethereum(module_ethereum::Action),
    Evm(module_evm::Action),
    Template(module_template::Action),
}

impl module_template::Config for BaseApp {}

impl module_account::Config for BaseApp {}

impl module_ethereum::Config for BaseApp {}

parameter_types! {
    pub const ChainId: u64 = 523;
    pub BlockGasLimit: U256 = U256::from(u32::max_value());
}

impl module_evm::Config for BaseApp {
    type AccountAsset = module_account::App<Self>;
    type AddressMapping = module_evm::impls::EthereumAddressMapping;
    type BlockGasLimit = BlockGasLimit;
    type ChainId = ChainId;
    type FeeCalculator = ();
    type OnChargeTransaction = module_evm::App<Self>;
    type Precompiles = ();
    type Runner = module_evm::runtime::runner::ActionRunner<Self>;
}

impl BaseApp {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let fdb = FinDB::open(base_dir)?;
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
}

impl ValidateUnsigned for BaseApp {
    type Call = Action;

    fn pre_execute(call: &Self::Call, _ctx: &Context) -> Result<()> {
        #[allow(unreachable_patterns)]
        match call {
            _ => Ok(()),
        }
    }

    fn validate_unsigned(call: &Self::Call, _ctx: &Context) -> Result<()> {
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
        ctx: &Context,
    ) -> Result<()> {
        match call {
            Action::Ethereum(action) => {
                module_ethereum::App::<Self>::execute(origin, action, ctx)
            }
            Action::Evm(action) => module_evm::App::<Self>::execute(origin, action, ctx),
            Action::Account(action) => {
                module_account::App::<Self>::execute(origin, action, ctx)
            }
            Action::Template(action) => {
                module_template::App::<Self>::execute(origin, action, ctx)
            }
        }
    }
}

impl BaseApp {
    pub fn create_query_context(&self, mut height: i64, prove: bool) -> Result<Context> {
        ensure!(
            height >= 0,
            "cannot query with height < 0; please provide a valid height"
        );

        // when a client did not provide a query height, manually inject the latest
        if height == 0 {
            height = self.chain_state.read().height()? as i64;
        }
        if height <= 1 && prove {
            return Err(eg!(
                "cannot query with proof when height <= 1; please provide a valid height"
            ));
        }

        let mut ctx = Context::new(self.chain_state.clone());
        ctx.header = self.check_state.header.clone();
        ctx.chain_id = self.check_state.header.chain_id.clone();
        ctx.check_tx = true;
        Ok(ctx)
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
        match mode {
            RunTxMode::Check => {
                ctx.check_tx = true;
                ctx.recheck_tx = false;
            }
            RunTxMode::ReCheck => {
                ctx.check_tx = true;
                ctx.recheck_tx = true;
            }
            _ => {
                ctx.check_tx = false;
                ctx.recheck_tx = false;
            }
        }
        ctx
    }

    fn validate_height(&self, height: i64) -> Result<()> {
        ensure!(height >= 1, format!("invalid height: {}", height));
        let mut expected_height =
            self.chain_state.read().height().unwrap_or_default() as i64;
        if expected_height == 0 && height > 1 {
            expected_height = height;
        } else {
            expected_height += 1;
        }
        ensure!(
            height == expected_height,
            format!("invalid height: {}; expected: {}", height, expected_height)
        );
        Ok(())
    }

    fn set_deliver_state(&mut self, header: Header) {
        self.deliver_state.check_tx = false;
        self.deliver_state.recheck_tx = false;
        self.deliver_state.header = header.clone();
        self.deliver_state.chain_id = header.chain_id;
    }

    fn set_check_state(&mut self, header: Header) {
        self.check_state.check_tx = true;
        self.deliver_state.recheck_tx = false;
        self.check_state.header = header.clone();
        self.check_state.chain_id = header.chain_id;
    }

    pub fn deliver_findora_tx(&mut self, tx: &FindoraTransaction) -> Result<()> {
        self.modules.process_findora_tx(&self.deliver_state, tx)
    }

    pub fn check_findora_tx(&mut self, tx: &FindoraTransaction) -> Result<()> {
        self.modules.process_findora_tx(&self.check_state, tx)
    }

    pub fn account_of(&self, addr: XfrPublicKey) -> Result<SmartAccount> {
        module_account::App::<BaseApp>::account_of(&self.deliver_state, &addr.into())
            .ok_or(eg!("account does not exist"))
    }
}
