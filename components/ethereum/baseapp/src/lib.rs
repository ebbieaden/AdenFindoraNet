mod app;

use app_evm::EvmModule;
use primitives::{crypto::*, module::AppModule, transaction, transaction::TxMsg};
use ruc::{eg, Result};
use std::collections::HashMap;

struct BaseApp {
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

pub enum Message {
    Ethereum(app_ethereum::Message),
    Evm(app_evm::Message),
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

    fn build_modules(&mut self, modules: Vec<Box<dyn AppModule>>) {
        for m in modules {
            self.modules.insert(m.name(), m);
        }
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

    pub fn handle_msg(&self, msg: Message) {
        match msg {
            Message::Ethereum(m) => {
                let am = self.modules.get(&m.route_path()).unwrap();
                am.tx_route(Box::new(m)).unwrap();
            }
            Message::Evm(m) => {
                let am = self.modules.get(&m.route_path()).unwrap();
                am.tx_route(Box::new(m)).unwrap();
            }
        }
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
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type Address = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedTransaction =
    transaction::UncheckedTransaction<Address, app_ethereum::Message, Signature>;
/// Extrinsic type that has already been checked.
pub type CheckedTransaction =
    transaction::CheckedTransaction<Address, app_ethereum::Message>;

pub struct EthereumTransactionConverter;

impl transaction::ConvertTransaction<UncheckedTransaction>
    for EthereumTransactionConverter
{
    fn convert_transaction(&self, transaction: &[u8]) -> Result<UncheckedTransaction> {
        let tx = serde_json::from_slice::<ethereum::Transaction>(transaction)
            .map_err(|e| eg!(e))?;
        Ok(UncheckedTransaction::new_unsigned(
            app_ethereum::Message::Transact(tx),
        ))
    }
}
