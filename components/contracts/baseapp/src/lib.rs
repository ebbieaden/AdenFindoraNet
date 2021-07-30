mod app;
mod extensions;
mod modules;
mod types;

use crate::modules::ModuleManager;
use abci::Header;
use fp_core::account::SmartAccount;
use fp_core::{
    account::MintOutput,
    context::{Context, RunTxMode},
    crypto::Address,
    ensure, parameter_types,
    transaction::{ActionResult, Executable, ValidateUnsigned},
};
use fp_traits::account::{AccountAsset, FeeCalculator};
use ledger::data_model::{Transaction as FindoraTransaction, TX_FEE_MIN};
use parking_lot::RwLock;
use primitive_types::{H160, H256, U256};
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use storage::{db::FinDB, state::ChainState};

pub use types::*;

const APP_NAME: &str = "findora";
const APP_DB_NAME: &str = "findora_db";
const CHAIN_STATE_PATH: &str = "chain.db";

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

pub struct StableTxFee;

impl FeeCalculator for StableTxFee {
    fn min_fee() -> u64 {
        TX_FEE_MIN
    }
}

impl module_account::Config for BaseApp {
    type FeeCalculator = StableTxFee;
}

impl module_ethereum::Config for BaseApp {}

parameter_types! {
    pub const ChainId: u64 = 523;
    pub BlockGasLimit: U256 = U256::from(u32::max_value());
}

impl module_evm::Config for BaseApp {
    type AccountAsset = module_account::App<Self>;
    type AddressMapping = fp_traits::evm::EthereumAddressMapping;
    type BlockGasLimit = BlockGasLimit;
    type BlockHashMapping = module_ethereum::App<Self>;
    type ChainId = ChainId;
    type DecimalsMapping = fp_traits::evm::EthereumDecimalsMapping;
    type FeeCalculator = ();
    type OnChargeTransaction = module_evm::App<Self>;
    type Precompiles = (
        evm_precompile_basic::ECRecover,
        evm_precompile_basic::Sha256,
        evm_precompile_basic::Ripemd160,
        evm_precompile_basic::Identity,
        evm_precompile_modexp::Modexp,
        evm_precompile_basic::ECRecoverPublicKey,
        evm_precompile_sha3fips::Sha3FIPS256,
        evm_precompile_sha3fips::Sha3FIPS512,
    );
    type Runner = module_evm::runtime::runner::ActionRunner<Self>;
}

impl BaseApp {
    pub fn new(base_dir: &Path) -> Result<Self> {
        // Creates a fresh chain state db
        let fdb_path = base_dir.clone().join(CHAIN_STATE_PATH);
        let fdb = FinDB::open(fdb_path.as_path())?;
        let chain_state =
            Arc::new(RwLock::new(ChainState::new(fdb, APP_DB_NAME.to_string())));

        Ok(BaseApp {
            name: APP_NAME.to_string(),
            version: "1.0.0".to_string(),
            app_version: 1,
            chain_state: chain_state.clone(),
            check_state: Context::new(chain_state.clone()),
            deliver_state: Context::new(chain_state),
            modules: ModuleManager::new(),
        })
    }
}

impl ValidateUnsigned for BaseApp {
    type Call = Action;

    fn pre_execute(_ctx: &Context, call: &Self::Call) -> Result<()> {
        #[allow(unreachable_patterns)]
        match call {
            _ => Ok(()),
        }
    }

    fn validate_unsigned(_ctx: &Context, call: &Self::Call) -> Result<()> {
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
        ctx.header_hash = self.check_state.header_hash.clone();
        ctx.chain_id = self.check_state.header.chain_id.clone();
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
        ctx.run_mode = mode;
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

    fn set_deliver_state(&mut self, header: Header, header_hash: Vec<u8>) {
        self.deliver_state.chain_id = header.chain_id.clone();
        self.deliver_state.header = header;
        self.deliver_state.header_hash = header_hash;
        self.deliver_state.run_mode = RunTxMode::None;
    }

    fn set_check_state(&mut self, header: Header, header_hash: Vec<u8>) {
        self.check_state.chain_id = header.chain_id.clone();
        self.check_state.header = header;
        self.check_state.header_hash = header_hash;
        self.check_state.run_mode = RunTxMode::None;
    }

    pub fn deliver_findora_tx(&mut self, tx: &FindoraTransaction) -> Result<()> {
        self.modules.process_findora_tx(&self.deliver_state, tx)
    }

    pub fn check_findora_tx(&mut self, tx: &FindoraTransaction) -> Result<()> {
        self.modules.process_findora_tx(&self.check_state, tx)
    }

    pub fn consume_mint(&mut self, size: usize) -> Result<Vec<MintOutput>> {
        self.modules.consume_mint(&self.deliver_state, size)
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
            module_evm::App::<Self>::account_codes(&ctx, &address)
        } else {
            None
        }
    }

    fn account_storage_at(&self, address: H160, index: H256) -> Option<H256> {
        if let Ok(ctx) = self.create_query_context(
            self.chain_state.read().height().unwrap_or_default(),
            false,
        ) {
            module_evm::App::<Self>::account_storages(&ctx, &address, &index)
        } else {
            None
        }
    }
}
