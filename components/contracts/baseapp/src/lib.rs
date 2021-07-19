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
    transaction::{ActionResult, Executable, ValidateUnsigned},
};
use fp_traits::account::AccountAsset;
use ledger::data_model::Transaction as FindoraTransaction;
use parking_lot::RwLock;
use primitive_types::{H160, U256};
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use storage::{db::FinDB, state::ChainState};

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
    type AddressMapping = fp_traits::evm::EthereumAddressMapping;
    type BlockGasLimit = BlockGasLimit;
    type ChainId = ChainId;
    type DecimalsMapping = fp_traits::evm::EthereumDecimalsMapping;
    type FeeCalculator = ();
    type OnChargeTransaction = module_evm::App<Self>;
    type Precompiles = ();
    type Runner = module_evm::runtime::runner::ActionRunner<Self>;
}

impl BaseApp {
    pub fn new(db_path: &Path) -> Result<Self> {
        // Before we can completely avoid replaying transactions
        // we need to clean db when restarting a node.
        if Path::exists(db_path) {
            fs::remove_dir_all(db_path).map_err(|_| eg!("Failed to remove db"))?;
        }

        // Creates a fresh chain state db
        let fdb = FinDB::open(db_path)?;
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
    ) -> Result<ActionResult> {
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
    pub fn create_query_context(&self, mut height: u64, prove: bool) -> Result<Context> {
        // when a client did not provide a query height, manually inject the latest
        if height == 0 {
            height = self.chain_state.read().height()?;
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

    fn set_check_state(&mut self, header: Header, header_hash: Vec<u8>) {
        self.check_state.check_tx = true;
        self.check_state.recheck_tx = false;
        self.check_state.header = header.clone();
        self.check_state.header_hash = header_hash;
        self.check_state.chain_id = header.chain_id;
    }

    pub fn deliver_findora_tx(&mut self, tx: &FindoraTransaction) -> Result<()> {
        self.modules.process_findora_tx(&self.deliver_state, tx)
    }

    pub fn check_findora_tx(&mut self, tx: &FindoraTransaction) -> Result<()> {
        self.modules.process_findora_tx(&self.check_state, tx)
    }
}

impl BaseProvider for BaseApp {
    fn account_of(&self, who: &Address, ctx: Option<Context>) -> Result<SmartAccount> {
        let ctx = match ctx {
            None => self.create_query_context(
                self.chain_state.read().height().unwrap_or_default(),
                false,
            )?,
            Some(ctx) => ctx,
        };
        module_account::App::<Self>::account_of(&ctx, who)
            .ok_or(eg!("account does not exist"))
    }

    fn current_block(&self) -> Option<ethereum::Block> {
        if let Ok(ctx) = self.create_query_context(
            self.chain_state.read().height().unwrap_or_default(),
            false,
        ) {
            module_ethereum::App::<Self>::current_block(&ctx)
        } else {
            None
        }
    }

    fn current_transaction_statuses(&self) -> Option<Vec<fp_evm::TransactionStatus>> {
        if let Ok(ctx) = self.create_query_context(
            self.chain_state.read().height().unwrap_or_default(),
            false,
        ) {
            module_ethereum::App::<Self>::current_transaction_statuses(&ctx)
        } else {
            None
        }
    }

    fn current_receipts(&self) -> Option<Vec<ethereum::Receipt>> {
        if let Ok(ctx) = self.create_query_context(
            self.chain_state.read().height().unwrap_or_default(),
            false,
        ) {
            module_ethereum::App::<Self>::current_receipts(&ctx)
        } else {
            None
        }
    }

    fn account_code_at(&self, address: H160) -> Option<Vec<u8>> {
        if let Ok(ctx) = self.create_query_context(
            self.chain_state.read().height().unwrap_or_default(),
            false,
        ) {
            module_evm::storage::AccountCodes::get(ctx.store, &address)
        } else {
            None
        }
    }
}
