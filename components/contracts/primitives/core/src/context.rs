use abci::Header;
pub use parking_lot::RwLock;
use protobuf::well_known_types::Timestamp;
pub use std::sync::Arc;
use storage::{
    db::FinDB,
    state::{ChainState, State},
};

pub type Store = State<FinDB>;

#[derive(Clone)]
pub struct Context {
    pub store: Arc<RwLock<Store>>,
    pub header: Header,
    pub header_hash: Vec<u8>,
    pub chain_id: String,
    pub tx: Vec<u8>,
    pub check_tx: bool,
    // if recheckTx == true, then checkTx must also be true
    pub recheck_tx: bool,
}

impl Context {
    pub fn new(cs: Arc<RwLock<ChainState<FinDB>>>) -> Self {
        Context {
            store: Arc::new(RwLock::new(Store::new(cs))),
            header: Default::default(),
            header_hash: vec![],
            chain_id: "".to_string(),
            tx: vec![],
            check_tx: false,
            recheck_tx: false,
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

    pub fn is_check_tx(&self) -> bool {
        self.check_tx
    }

    pub fn is_recheck_tx(&self) -> bool {
        self.recheck_tx
    }
}
