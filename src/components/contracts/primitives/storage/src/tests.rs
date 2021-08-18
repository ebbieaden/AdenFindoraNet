use crate::hash::{Sha256, StorageHasher};
use crate::*;
use sha2::Digest;
use std::env::temp_dir;
use std::time::SystemTime;
use storage::db::TempFinDB;
use storage::state::{ChainState, State};

#[test]
fn storage_hasher_works() {
    let text = b"hello world";

    assert_eq!(sha2::Sha256::digest(text).as_slice(), Sha256::hash(text));
}

fn create_temp_db() -> Arc<RwLock<State<TempFinDB>>> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = temp_dir();
    path.push(format!("temp-findora-db–{}", time));
    let fdb = TempFinDB::open(path).unwrap();
    let chain_state = Arc::new(RwLock::new(ChainState::new(fdb, "temp_db".to_string())));
    Arc::new(RwLock::new(State::new(chain_state)))
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
    assert!(Number::exists(store.clone()));
    Number::delete(store.clone());
    assert_eq!(Number::get(store.clone()), None);
    assert!(!Number::exists(store));
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
    assert!(Account::contains_key(store.clone(), &"a".to_string()));
    Account::remove(store.clone(), &"a".to_string());
    assert_eq!(Account::get(store.clone(), &"a".to_string()), None);
    assert!(!Account::contains_key(store.clone(), &"a".to_string()),);

    let kvs = Account::iterate(store.clone());
    assert_eq!(kvs, vec![("b".to_string(), 20), ("c".to_string(), 30)]);

    store.write().commit(1).unwrap();
    let kvs = Account::iterate(store);
    assert_eq!(kvs, vec![("b".to_string(), 20), ("c".to_string(), 30)]);
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
    assert!(Data::contains_key(store.clone(), &1, &2));
    Data::remove(store.clone(), &1, &2);
    assert_eq!(Data::get(store.clone(), &1, &2), None);
    assert!(!Data::contains_key(store.clone(), &1, &2));

    let kvs = Data::iterate_prefix(store.clone(), &1);
    assert_eq!(kvs, vec![(3, 20)]);

    let kvs = Data::iterate_prefix(store.clone(), &2);
    assert_eq!(kvs, vec![(3, 30), (4, 40)]);

    Data::remove_prefix(store.clone(), &2);
    let kvs = Data::iterate_prefix(store.clone(), &2);
    assert_eq!(kvs, vec![]);

    store.write().commit(1).unwrap();
    let kvs = Data::iterate_prefix(store, &1);
    assert_eq!(kvs, vec![(3, 20)]);
}
