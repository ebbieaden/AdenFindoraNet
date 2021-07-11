use parking_lot::RwLock;
use std::{env::temp_dir, path::PathBuf, sync::Arc, time::SystemTime};
use storage::{db::FinDB, state::ChainState};

#[allow(unused)]
pub fn create_temp_db() -> Arc<RwLock<ChainState<FinDB>>> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = temp_dir();
    path.push(format!("temp-findora-db–{}", time));
    let fdb = FinDB::open(path).unwrap();
    Arc::new(RwLock::new(ChainState::new(fdb, "temp_db".to_string())))
}

pub fn create_temp_db_path() -> PathBuf {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = temp_dir();
    path.push(format!("temp-findora-db–{}", time));
    path
}
