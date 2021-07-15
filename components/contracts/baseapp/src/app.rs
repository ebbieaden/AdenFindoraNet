use crate::{types::convert_unchecked_transaction, RunTxMode};
use abci::*;
use fp_core::context::Context;
use ruc::{pnk, RucResult};

impl Application for crate::BaseApp {
    // ignore
    /// info implements the ABCI interface.
    fn info(&mut self, _req: &RequestInfo) -> ResponseInfo {
        let mut info = ResponseInfo::new();
        info.set_data(self.name.clone());
        info.set_version(self.version.clone());
        info.set_app_version(self.app_version);
        let _ = self
            .chain_state
            .read()
            .height()
            .map(|h| info.set_last_block_height(h as i64));

        info
    }

    /// query implements the ABCI interface.
    fn query(&mut self, req: &RequestQuery) -> ResponseQuery {
        let err_resp = |err: String| -> ResponseQuery {
            let mut resp = ResponseQuery::new();
            resp.set_code(1);
            resp.set_log(err);
            resp
        };

        if req.height < 0 {
            return err_resp(
                "cannot query with height < 0; please provide a valid height"
                    .to_string(),
            );
        }

        // example: "module/evm/code"
        let mut path: Vec<_> = req.path.split('/').collect();
        if 0 == path.len() {
            return err_resp("Empty query path!".to_string());
        }

        let ctx = self.create_query_context(req.height as u64, req.prove);
        if let Err(e) = ctx {
            return err_resp(format!("Cannot create query context with err: {}!", e));
        }

        match path.remove(0) {
            // "store" => self.store.query(path, req),
            "module" => self.modules.query(ctx.unwrap(), path, req),
            _ => err_resp("Invalid query path!".to_string()),
        }
    }

    /// check_tx implements the ABCI interface and executes a tx in Check/ReCheck mode.
    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        let mut resp = ResponseCheckTx::new();
        if let Ok(tx) = convert_unchecked_transaction(req.get_tx()) {
            let check_fn = |mode: RunTxMode| {
                let ctx = self.retrieve_context(mode, req.get_tx().to_vec()).clone();
                if let Err(e) = self.modules.process_tx(ctx, mode, tx) {
                    resp.set_code(1);
                    resp.set_log(format!("Ethereum transaction check failed: {}", e));
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

    /// init_chain implements the ABCI interface.
    fn init_chain(&mut self, req: &RequestInitChain) -> ResponseInitChain {
        let mut init_header = Header::new();
        init_header.chain_id = req.chain_id.clone();
        init_header.time = req.time.clone();

        // initialize the deliver state and check state with a correct header
        self.set_deliver_state(init_header.clone());
        self.set_check_state(init_header, vec![]);

        // TODO init genesis about consensus and validators

        ResponseInitChain::new()
    }

    /// begin_block implements the ABCI application interface.
    fn begin_block(&mut self, req: &RequestBeginBlock) -> ResponseBeginBlock {
        pnk!(self.validate_height(req.header.as_ref().unwrap_or_default().height));

        // Initialize the DeliverTx state. If this is the first block, it should
        // already be initialized in InitChain. Otherwise app.deliverState will be
        // nil, since it is reset on Commit.
        self.set_deliver_state(req.header.as_ref().unwrap_or_default().clone());
        self.deliver_state.header_hash = req.hash.clone();

        self.modules.begin_block(&mut self.deliver_state, req);

        ResponseBeginBlock::new()
    }

    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        let mut resp = ResponseDeliverTx::new();
        if let Ok(tx) = convert_unchecked_transaction(req.get_tx()) {
            // TODO event
            let ctx = self
                .retrieve_context(RunTxMode::Deliver, req.get_tx().to_vec())
                .clone();

            let ret = self.modules.process_tx(ctx, RunTxMode::Deliver, tx);
            match ret {
                Ok(ar) => {
                    resp.set_data(ar.data);
                    resp.set_log(ar.log);
                    resp.set_gas_wanted(ar.gas_wanted as i64);
                    resp.set_gas_used(ar.gas_used as i64);
                    resp.set_events(protobuf::RepeatedField::from_vec(ar.events));
                    resp
                }
                Err(e) => {
                    resp.set_code(1);
                    resp.set_log(format!("Failed to deliver transaction: {}!", e));
                    resp
                }
            }
        } else {
            resp.set_code(1);
            resp.set_log(String::from(
                "Failed to convert transaction when deliver tx!",
            ));
            resp
        }
    }

    fn end_block(&mut self, req: &RequestEndBlock) -> ResponseEndBlock {
        self.modules.end_block(&mut self.deliver_state, req);
        ResponseEndBlock::new()
    }

    fn commit(&mut self, _req: &RequestCommit) -> ResponseCommit {
        let header = self.deliver_state.block_header();
        let header_hash = self.deliver_state.header_hash();

        // Write the DeliverTx state into branched storage and commit the Store.
        // The write to the DeliverTx state writes all state transitions to the root
        // Store so when commit() is called is persists those values.
        let _ = self
            .deliver_state
            .store
            .write()
            .commit(header.height as u64);

        // Reset the Check state to the latest committed.
        self.set_check_state(header, header_hash);
        // Reset the deliver state
        self.deliver_state = Context::new(self.chain_state.clone());

        let mut res = ResponseCommit::new();
        res.set_data(self.chain_state.read().root_hash());
        res
    }
}
