use crate::{types::convert_ethereum_transaction, RunTxMode};
use abci::*;
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

        // example: "/module/evm/code"
        let mut path: Vec<_> = req.path.split('/').collect();
        if 0 == path.len() {
            return err_resp("Empty query path !".to_string());
        }

        let ctx = self.create_query_context(req.height, req.prove);
        if let Err(e) = ctx {
            return err_resp(format!("Cannot create query context with err: {}!", e));
        }

        match path.remove(0) {
            // "store" => self.store.query(path, req),
            "module" => self.modules.query(ctx.unwrap(), path, req),
            _ => err_resp("Invalid query path!".to_string()),
        }
    }

    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        let mut resp = ResponseCheckTx::new();
        if let Ok(tx) = convert_ethereum_transaction(req.get_tx()) {
            let check_fn = |mode: RunTxMode| {
                let ctx = self.retrieve_context(mode, req.get_tx().to_vec()).clone();
                if ruc::info!(self.modules.process_tx(ctx, mode, tx)).is_err() {
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
        self.deliver_state.header = init_header.clone();
        self.check_state.header = init_header;

        ResponseInitChain::new()
    }

    fn begin_block(&mut self, req: &RequestBeginBlock) -> ResponseBeginBlock {
        self.modules.begin_block(&mut self.deliver_state, req);
        ResponseBeginBlock::new()
    }

    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        let mut resp = ResponseDeliverTx::new();
        if let Ok(tx) = convert_ethereum_transaction(req.get_tx()) {
            // TODO event
            let ctx = self
                .retrieve_context(RunTxMode::Deliver, req.get_tx().to_vec())
                .clone();
            if self.modules.process_tx(ctx, RunTxMode::Deliver, tx).is_ok() {
                return resp;
            }
        }
        resp.set_code(1);
        resp.set_log(String::from("Failed to deliver transaction!"));
        resp
    }

    fn end_block(&mut self, req: &RequestEndBlock) -> ResponseEndBlock {
        self.modules.end_block(&mut self.deliver_state, req);
        ResponseEndBlock::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        ResponseCommit::new()
    }
}
