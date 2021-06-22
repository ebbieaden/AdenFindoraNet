mod app;

use app_evm::EvmModule;
use primitives::{message, message::TxMsg, module::AppModule};
use ruc::Result;
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
    EVM(message::evm::Message),
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
        // app.register_routes();
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
            Message::EVM(m) => {
                let am = self.modules.get(&m.route_path()).unwrap();
                am.tx_route(Box::new(m));
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
        match module_name {
            message::evm::EVM_MODULE_NAME => {
                let am = self
                    .modules
                    .get(&message::evm::EVM_MODULE_NAME.to_string())
                    .unwrap();
                am.query_route(path, req)
            }
            _ => {
                resp.set_code(1);
                resp.set_log(format!("Invalid query module route: {}!", module_name));
                resp
            }
        }
    }
}
