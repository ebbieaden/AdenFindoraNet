use abci::Header;
pub use parking_lot::RwLock;
use protobuf::well_known_types::Timestamp;
pub use std::sync::Arc;
use storage::{
    db::FinDB,
    state::{ChainState, SessionedCache, State},
};

pub type CacheStore = SessionedCache;

pub type CommitStore = State<FinDB>;

pub struct CacheState {
    pub sc: SessionedCache,
    pub ctx: Context,
}

impl CacheState {
    pub fn new(cs: Arc<RwLock<ChainState<FinDB>>>) -> Self {
        CacheState {
            sc: SessionedCache::new(),
            ctx: Context::new(Arc::new(RwLock::new(CommitStore::new(cs)))),
        }
    }
}

pub struct CommitState {
    pub cs: CommitStore,
    pub ctx: Context,
}

impl CommitState {
    pub fn new(cs: Arc<RwLock<ChainState<FinDB>>>) -> Self {
        CommitState {
            cs: CommitStore::new(cs.clone()),
            ctx: Context::new(Arc::new(RwLock::new(CommitStore::new(cs)))),
        }
    }
}

#[derive(Clone)]
pub struct Context {
    pub store: Arc<RwLock<CommitStore>>,
    pub header: Header,
    pub header_hash: Vec<u8>,
    pub chain_id: String,
    pub tx: Vec<u8>,
    pub check_tx: bool,
    // if recheckTx == true, then checkTx must also be true
    pub recheck_tx: bool,
}

impl Context {
    pub fn new(cs: Arc<RwLock<CommitStore>>) -> Self {
        Context {
            store: cs,
            header: Default::default(),
            header_hash: vec![],
            chain_id: "".to_string(),
            tx: vec![],
            check_tx: false,
            recheck_tx: false,
        }
    }

    pub fn with_block_header(mut self, header: Header) -> Self {
        self.header = header;
        self
    }

    pub fn with_header_hash(mut self, hash: Vec<u8>) -> Self {
        self.header_hash = hash;
        self
    }

    pub fn with_chain_id(mut self, id: String) -> Self {
        self.chain_id = id;
        self
    }

    pub fn with_tx(mut self, tx: Vec<u8>) -> Self {
        self.tx = tx;
        self
    }

    pub fn with_check_tx(mut self, check_tx: bool) -> Self {
        self.check_tx = check_tx;
        self
    }

    pub fn with_recheck_tx(mut self, recheck_tx: bool) -> Self {
        self.recheck_tx = recheck_tx;
        self
    }
}

impl Context {
    pub fn commit_store(&self) -> Arc<RwLock<CommitStore>> {
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
