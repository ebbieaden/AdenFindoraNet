use abci::Header;
pub use parking_lot::RwLock;
use protobuf::well_known_types::Timestamp;
pub use std::sync::Arc;
use storage::{
    db::FinDB,
    state::{ChainState, State},
};

pub type Store = State<FinDB>;

#[derive(Clone, PartialEq, Eq, Debug, Hash, Copy)]
pub enum RunTxMode {
    None = 0,
    /// Check a transaction
    Check = 1,
    /// Recheck a (pending) transaction after a commit
    ReCheck = 2,
    /// Simulate a transaction
    Simulate = 3,
    /// Deliver a transaction
    Deliver = 4,
}

#[derive(Clone)]
pub struct Context {
    pub store: Arc<RwLock<Store>>,
    pub header: Header,
    pub header_hash: Vec<u8>,
    pub chain_id: String,
    pub tx: Vec<u8>,
    pub run_mode: RunTxMode,
}

impl Context {
    pub fn new(cs: Arc<RwLock<ChainState<FinDB>>>) -> Self {
        Context {
            store: Arc::new(RwLock::new(Store::new(cs))),
            header: Default::default(),
            header_hash: vec![],
            chain_id: "".to_string(),
            tx: vec![],
            run_mode: RunTxMode::None,
        }
    }
}

impl Context {
    pub fn commit_store(&self) -> Arc<RwLock<Store>> {
        self.store.clone()
    }

    pub fn block_header(&self) -> Header {
        self.header.clone()
    }

    pub fn header_hash(&self) -> Vec<u8> {
        self.header_hash.clone()
    }

    pub fn block_height(&self) -> i64 {
        self.header.get_height()
    }

    pub fn block_time(&self) -> &Timestamp {
        self.header.get_time()
    }

    pub fn chain_id(&self) -> String {
        self.chain_id.clone()
    }

    pub fn tx(&self) -> Vec<u8> {
        self.tx.clone()
    }

    pub fn run_mode(&self) -> RunTxMode {
        self.run_mode
    }
}
