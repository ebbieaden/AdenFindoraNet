use abci::*;

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

    fn check_tx(&mut self, _req: &RequestCheckTx) -> ResponseCheckTx {
        ResponseCheckTx::new()
    }

    fn init_chain(&mut self, _req: &RequestInitChain) -> ResponseInitChain {
        ResponseInitChain::new()
    }

    fn begin_block(&mut self, _req: &RequestBeginBlock) -> ResponseBeginBlock {
        ResponseBeginBlock::new()
    }

    fn deliver_tx(&mut self, _p: &RequestDeliverTx) -> ResponseDeliverTx {
        ResponseDeliverTx::new()
    }

    fn end_block(&mut self, _req: &RequestEndBlock) -> ResponseEndBlock {
        ResponseEndBlock::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        ResponseCommit::new()
    }
}
