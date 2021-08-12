use crate::extensions::SignedExtra;
use fp_core::context::{Context, RunTxMode};
use fp_types::assemble::convert_unchecked_transaction;
use log::{debug, error};
use ruc::*;
use tendermint_sys::SyncApplication;
use tm_protos::abci::*;

impl SyncApplication for crate::BaseApp {
    /// info implements the ABCI interface.
    /// - Returns chain info (las height and hash where the node left off)
    /// - Tendermint uses info to decide from which height/hash to continue
    fn info(&mut self, _req: RequestInfo) -> ResponseInfo {
        let mut info: ResponseInfo = Default::default();
        info.data = self.name.clone();
        info.version = self.version.clone();
        info.app_version = self.app_version;

        let height = self.chain_state.read().height().unwrap();
        let hash = self.chain_state.read().root_hash();
        info.last_block_height = height as i64;
        info.last_block_app_hash = hash;

        info
    }

    /// init_chain implements the ABCI interface.
    fn init_chain(&mut self, req: RequestInitChain) -> ResponseInitChain {
        let mut init_header: Header = Default::default();
        init_header.chain_id = req.chain_id.clone();
        init_header.time = req.time;

        // initialize the deliver state and check state with a correct header
        Self::update_state(&mut self.deliver_state, init_header.clone(), vec![]);
        Self::update_state(&mut self.check_state, init_header, vec![]);

        // TODO init genesis about consensus and validators

        ResponseInitChain::default()
    }

    /// query implements the ABCI interface.
    fn query(&mut self, req: RequestQuery) -> ResponseQuery {
        let err_resp = |err: String| -> ResponseQuery {
            let mut resp: ResponseQuery = Default::default();
            resp.code = 1;
            resp.log = err;
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
        if path.is_empty() {
            return err_resp("Empty query path!".to_string());
        }

        let ctx = self.create_query_context(req.height as u64, req.prove);
        if let Err(e) = ctx {
            return err_resp(format!("Cannot create query context with err: {}!", e));
        }

        match path.remove(0) {
            // "store" => self.store.query(path, req),
            "module" => self.modules.query(ctx.unwrap(), path, &req),
            _ => err_resp("Invalid query path!".to_string()),
        }
    }

    /// check_tx implements the ABCI interface and executes a tx in Check/ReCheck mode.
    fn check_tx(&mut self, req: RequestCheckTx) -> ResponseCheckTx {
        let mut resp: ResponseCheckTx = Default::default();
        if let Ok(tx) = convert_unchecked_transaction::<SignedExtra>(&req.tx) {
            let check_fn = |mode: RunTxMode| {
                let ctx = self.retrieve_context(mode, req.tx.clone()).clone();
                if let Err(e) = self.modules.process_tx::<SignedExtra>(ctx, tx) {
                    debug!(target: "baseapp", "Transaction check error: {}", e);
                    resp.code = 1;
                    resp.log = format!("Transaction check error: {}", e);
                }
            };
            match req.r#type() {
                CheckTxType::New => check_fn(RunTxMode::Check),
                CheckTxType::Recheck => check_fn(RunTxMode::ReCheck),
            }
        } else {
            resp.code = 1;
            resp.log = String::from("Could not unpack transaction");
        }
        resp
    }

    /// begin_block implements the ABCI application interface.
    fn begin_block(&mut self, req: RequestBeginBlock) -> ResponseBeginBlock {
        pnk!(self.validate_height(req.header.clone().unwrap_or_default().height));

        // Initialize the DeliverTx state. If this is the first block, it should
        // already be initialized in InitChain. Otherwise app.deliverState will be
        // nil, since it is reset on Commit.
        Self::update_state(
            &mut self.deliver_state,
            req.header.clone().unwrap_or_default(),
            req.hash.clone(),
        );

        self.modules.begin_block(&mut self.deliver_state, &req);

        ResponseBeginBlock::default()
    }

    fn deliver_tx(&mut self, req: RequestDeliverTx) -> ResponseDeliverTx {
        let mut resp: ResponseDeliverTx = Default::default();
        if let Ok(tx) = convert_unchecked_transaction::<SignedExtra>(&req.tx) {
            let ctx = self.retrieve_context(RunTxMode::Deliver, req.tx).clone();

            let ret = self.modules.process_tx::<SignedExtra>(ctx, tx);
            match ret {
                Ok(ar) => {
                    debug!(target: "baseapp", "deliver tx succeed result: {:?}", ar);

                    resp.data = ar.data;
                    resp.log = ar.log;
                    resp.gas_wanted = ar.gas_wanted as i64;
                    resp.gas_used = ar.gas_used as i64;
                    resp.events = ar.events;
                    resp
                }
                Err(e) => {
                    error!(target: "baseapp", "Ethereum transaction deliver error: {}", e);
                    resp.code = 1;
                    resp.log = format!("Ethereum transaction deliver error: {}", e);
                    resp
                }
            }
        } else {
            resp.code = 1;
            resp.log = String::from("Failed to convert transaction when deliver tx!");
            resp
        }
    }

    #[cfg(not(test))]
    fn end_block(&mut self, req: RequestEndBlock) -> ResponseEndBlock {
        self.modules.end_block(&mut self.deliver_state, &req)
    }

    #[cfg(test)]
    fn end_block(&mut self, req: RequestEndBlock) -> ResponseEndBlock {
        Default::default()
    }

    fn commit(&mut self) -> ResponseCommit {
        let header = self.deliver_state.block_header();
        let header_hash = self.deliver_state.header_hash();

        self.check_state.store = self.deliver_state.store.clone();

        // Write the DeliverTx state into branched storage and commit the Store.
        // The write to the DeliverTx state writes all state transitions to the root
        // Store so when commit() is called is persists those values.
        let (root_hash, _) = self
            .deliver_state
            .store
            .write()
            .commit(header.height as u64)
            .unwrap_or_else(|_| {
                panic!("Failed to commit chain state at height: {}", header.height)
            });

        // Reset the Check state to the latest committed.
        Self::update_state(&mut self.check_state, header.clone(), header_hash);

        // Reset the deliver state
        self.deliver_state = Context::new(self.chain_state.clone());

        // Set root hash
        let mut res: ResponseCommit = Default::default();
        res.data = root_hash;
        res
    }
}
