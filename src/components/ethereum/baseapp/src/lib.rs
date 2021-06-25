mod app;
mod types;

use app_ethereum::EthereumModule;
use app_evm::EvmModule;
use primitives::{
    crypto::*,
    module::AppModule,
    transaction::{Applyable, Executable, ValidateUnsigned},
};
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use types::*;

pub struct BaseApp {
    // application name from abci.Info
    name: String,
    // application's version string
    version: String,
    // application's protocol version that increments on every upgrade
    // if BaseApp is passed to the upgrade keeper's NewKeeper method.
    app_version: u64,
    // manage all modules
    modules: HashMap<String, Box<dyn AppModule>>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum RunTxMode {
    // Check a transaction
    Check = 0,
    // Recheck a (pending) transaction after a commit
    ReCheck = 1,
    // Simulate a transaction
    Simulate = 2,
    // Deliver a transaction
    Deliver = 3,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Ethereum(app_ethereum::Action),
    Evm(app_evm::Action),
}

impl Executable for Action {
    type Origin = Address;

    fn execute(self, origin: Option<Self::Origin>) -> Result<()> {
        match self {
            Action::Ethereum(action) => action.execute(origin),
            Action::Evm(action) => action.execute(origin),
        }
    }
}

impl BaseApp {
    pub fn new() -> Self {
        let mut app = BaseApp {
            name: "findora".to_string(),
            version: "1.0.0".to_string(),
            app_version: 1,
            modules: HashMap::new(),
        };

        app.build_modules(vec![Box::new(EvmModule::new())]);
        app
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
        if let Some(am) = self.modules.get(&module_name.to_string()) {
            am.query_route(path, req)
        } else {
            resp.set_code(1);
            resp.set_log(format!("Invalid query module route: {}!", module_name));
            resp
        }
    }

    pub fn handle_tx(&self, mode: RunTxMode, tx: UncheckedTransaction) -> Result<()> {
        let checked = tx.clone().check()?;

        match tx.function {
            Action::Ethereum(action) => self
                .dispatch::<app_ethereum::Action, EthereumModule>(mode, action, checked),
            Action::Evm(_) => {
                self.dispatch::<Action, BaseApp>(mode, tx.function, checked)
            }
        }
    }
}

impl BaseApp {
    fn build_modules(&mut self, modules: Vec<Box<dyn AppModule>>) {
        for m in modules {
            self.modules.insert(m.name(), m);
        }
    }

    fn dispatch<
        Call: Executable<Origin = Address>,
        Module: ValidateUnsigned<Call = Call>,
    >(
        &self,
        mode: RunTxMode,
        action: Call,
        tx: CheckedTransaction,
    ) -> Result<()> {
        // TODO gas check、get ctx.store

        let origin_tx = convert_unsigned_transaction::<Call>(action, tx);

        origin_tx.validate::<Module>()?;

        if mode == RunTxMode::Deliver {
            origin_tx.apply::<Module>()?;
        }
        Ok(())
    }
}

impl ValidateUnsigned for BaseApp {
    type Call = Action;

    fn pre_execute(call: &Self::Call) -> Result<()> {
        #[allow(unreachable_patterns)]
        match call {
            _ => Ok(()),
        }
    }

    fn validate_unsigned(call: &Self::Call) -> Result<()> {
        #[allow(unreachable_patterns)]
        match call {
            _ => Err(eg!(
                "Could not find an unsigned validator for the unsigned transaction"
            )),
        }
    }
}
