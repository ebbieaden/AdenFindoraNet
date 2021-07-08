use crate::hash::{Sha256, StorageHasher};
use crate::*;
use fp_core::context::Store;
use sha2::Digest;
use std::env::temp_dir;
use std::time::SystemTime;
use storage::db::FinDB;
use storage::state::ChainState;

#[test]
fn storage_hasher_works() {
    let text = b"hello world";

    assert_eq!(sha2::Sha256::digest(text).as_slice(), Sha256::hash(text));
}

fn create_temp_db() -> Arc<RwLock<Store>> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = temp_dir();
    path.push(format!("temp-findora-db–{}", time));
    let fdb = FinDB::open(path).unwrap();
    let chain_state = Arc::new(RwLock::new(ChainState::new(fdb, "temp_db".to_string())));
    Arc::new(RwLock::new(Store::new(chain_state)))
}

#[test]
fn storage_value_works() {
    generate_storage!(Findora, Number => Value<u32>);

    assert_eq!(Number::module_prefix(), b"Findora");
    assert_eq!(Number::storage_prefix(), b"Number");
    assert_eq!(
        Number::hashed_key(),
        sha2::Sha256::digest(b"FindoraNumber").as_slice()
    );

    let store = create_temp_db();
    Number::put(store.clone(), 10);

    assert_eq!(Number::get(store.clone()), Some(10));
    assert_eq!(Number::exists(store.clone()), true);
    Number::delete(store.clone());
    assert_eq!(Number::get(store.clone()), None);
    assert_eq!(Number::exists(store), false);
}

#[test]
fn storage_map_test() {
    generate_storage!(Findora, Account => Map<String, u32>);

    assert_eq!(Account::module_prefix(), b"Findora");
    assert_eq!(Account::storage_prefix(), b"Account");

    let store = create_temp_db();
    Account::insert(store.clone(), &"a".to_string(), &10);
    Account::insert(store.clone(), &"b".to_string(), &20);
    Account::insert(store.clone(), &"c".to_string(), &30);

    assert_eq!(Account::get(store.clone(), &"a".to_string()), Some(10));
    assert_eq!(Account::contains_key(store.clone(), &"a".to_string()), true);
    Account::remove(store.clone(), &"a".to_string());
    assert_eq!(Account::get(store.clone(), &"a".to_string()), None);
    assert_eq!(
        Account::contains_key(store.clone(), &"a".to_string()),
        false
    );

    let kvs = Account::iterate(store.clone());
    assert_eq!(kvs, vec![("b".to_string(), 20), ("c".to_string(), 30)]);

    // TODO fix
    // store.write().commit(1).unwrap();
    // let kvs = Account::iterate(store.clone());
    // assert_eq!(kvs, vec![("b".to_string(), 20), ("c".to_string(), 30)]);
}

#[test]
fn storage_double_map_test() {
    generate_storage!(Findora, Data => DoubleMap<u32, u32, u32>);

    assert_eq!(Data::module_prefix(), b"Findora");
    assert_eq!(Data::storage_prefix(), b"Data");

    let store = create_temp_db();
    Data::insert(store.clone(), &1, &2, &10);
    Data::insert(store.clone(), &1, &3, &20);
    Data::insert(store.clone(), &2, &3, &30);
    Data::insert(store.clone(), &2, &4, &40);

    assert_eq!(Data::get(store.clone(), &1, &2), Some(10));
    assert_eq!(Data::contains_key(store.clone(), &1, &2), true);
    Data::remove(store.clone(), &1, &2);
    assert_eq!(Data::get(store.clone(), &1, &2), None);
    assert_eq!(Data::contains_key(store.clone(), &1, &2), false);

    let kvs = Data::iterate_prefix(store.clone(), &1);
    assert_eq!(kvs, vec![(3, 20)]);

    let kvs = Data::iterate_prefix(store.clone(), &2);
    assert_eq!(kvs, vec![(3, 30), (4, 40)]);

    Data::remove_prefix(store.clone(), &2);
    let kvs = Data::iterate_prefix(store.clone(), &2);
    assert_eq!(kvs, vec![]);

    // TODO fix
    // store.write().commit(1).unwrap();
    // let kvs = Data::iterate_prefix(store.clone(), 1);
    // assert_eq!(kvs, vec![(3, 20)]);
}
