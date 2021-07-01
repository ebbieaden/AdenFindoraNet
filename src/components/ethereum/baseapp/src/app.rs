use crate::{types::convert_ethereum_transaction, RunTxMode};
use abci::*;
use primitives::module::AppModule;
use ruc::RucResult;

impl Application for crate::BaseApp {
    fn info(&mut self, _req: &RequestInfo) -> ResponseInfo {
        let mut info = ResponseInfo::new();
        info.set_data(self.name());
        info.set_version(self.version());
        info.set_app_version(self.app_version());
        info
    }

    fn query(&mut self, req: &RequestQuery) -> ResponseQuery {
        let err_resp = |err: String| -> ResponseQuery {
            let mut resp = ResponseQuery::new();
            resp.set_code(1);
            resp.set_log(err);
            resp
        };

        // example: "/custom/evm/code"
        let mut path: Vec<_> = req.path.split('/').collect();
        if 0 == path.len() {
            return err_resp("Empty query path !".to_string());
        }

        match path.remove(0) {
            "app" => self.handle_query(path, req),
            "store" => self.handle_query(path, req),
            "custom" => self.handle_query(path, req),
            _ => err_resp("Invalid query path!".to_string()),
        }
    }

    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        let mut resp = ResponseCheckTx::new();
        if let Ok(tx) = convert_ethereum_transaction(req.get_tx()) {
            let check_fn = |mode: RunTxMode| {
                if ruc::info!(self.handle_tx(mode, tx, req.get_tx().to_vec())).is_err() {
                    resp.set_code(1);
                    resp.set_log(String::from("Ethereum transaction check failed"));
                }
            };
            match req.get_field_type() {
                CheckTxType::New => check_fn(RunTxMode::Check),
                CheckTxType::Recheck => check_fn(RunTxMode::ReCheck),
            }
        } else {
            resp.set_code(1);
            resp.set_log(String::from("Could not unpack transaction"));
        }
        resp
    }

    fn init_chain(&mut self, req: &RequestInitChain) -> ResponseInitChain {
        // On a new chain, we consider the init chain block height as 0, even though
        // req.InitialHeight is 1 by default.
        let mut init_header = Header::new();
        init_header.chain_id = req.chain_id.clone();
        init_header.time = req.time.clone();

        // initialize the deliver state and check state with a correct header
        self.set_deliver_state(init_header.clone());
        self.set_check_state(init_header);

        ResponseInitChain::new()
    }

    fn begin_block(&mut self, req: &RequestBeginBlock) -> ResponseBeginBlock {
        // for m in self.modules.iter_mut() {
        //     m.begin_block(req);
        // }
        self.ethereum_module.begin_block(req);
        self.evm_module.begin_block(req);

        ResponseBeginBlock::new()
    }

    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        let mut resp = ResponseDeliverTx::new();
        if let Ok(tx) = convert_ethereum_transaction(req.get_tx()) {
            // TODO events、storage
            if self
                .handle_tx(RunTxMode::Deliver, tx, req.get_tx().to_vec())
                .is_ok()
            {
                return resp;
            }
        }
        resp.set_code(1);
        resp.set_log(String::from("Failed to deliver transaction!"));
        resp
    }

    fn end_block(&mut self, req: &RequestEndBlock) -> ResponseEndBlock {
        // for m in self.modules.iter_mut() {
        //     m.end_block(req);
        // }
        self.ethereum_module.end_block(req);
        self.evm_module.end_block(req);

        ResponseEndBlock::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        ResponseCommit::new()
    }
}
