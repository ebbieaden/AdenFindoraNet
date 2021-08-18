use crate::hash::StorageHasher;
use crate::*;
use ruc::*;
use std::str::FromStr;
use storage::db::{IterOrder, MerkleDB};
use storage::state::{KVecMap, State};

/// A type that allow to store values for `(key1, key2)` couple. Similar to `StorageMap` but allow
/// to iterate and remove value associated to first key.
///
/// Each value is stored at:
/// ```nocompile
/// Sha256(Prefix::module_prefix() + Prefix::STORAGE_PREFIX)
///    ++ serialize(key1)
///    ++ serialize(key2)
/// ```
///
pub struct StorageDoubleMap<Prefix, Hasher, Key1, Key2, Value>(
    core::marker::PhantomData<(Prefix, Hasher, Key1, Key2, Value)>,
);

impl<Prefix, Hasher, Key1, Key2, Value>
    StorageDoubleMap<Prefix, Hasher, Key1, Key2, Value>
where
    Prefix: StorageInstance,
    Hasher: StorageHasher<Output = [u8; 32]>,
    Key1: ToString + FromStr,
    Key2: ToString + FromStr,
    Value: Serialize + DeserializeOwned,
{
    pub fn module_prefix() -> &'static [u8] {
        Prefix::module_prefix().as_bytes()
    }

    pub fn storage_prefix() -> &'static [u8] {
        Prefix::STORAGE_PREFIX.as_bytes()
    }

    /// Get the storage key used to fetch a value corresponding to a specific key.
    pub fn build_key_for(k1: &Key1, k2: &Key2) -> Vec<u8> {
        let prefix_key: Vec<u8> =
            [Self::module_prefix(), Self::storage_prefix()].concat();
        let data_key1 = k1.to_string();
        let data_key2 = k2.to_string();

        let final_key = storage::store::Prefix::new(prefix_key.as_slice());
        final_key
            .push_sub(data_key1.as_ref(), data_key2.as_ref())
            .as_ref()
            .to_vec()
    }

    pub fn parse_key_for(key_list: Vec<&str>) -> Result<Key2> {
        let last_key = key_list
            .last()
            .copied()
            .ok_or(eg!("parse key failed with empty list"))?;
        Key2::from_str(last_key).map_err(|_| eg!("key convert to string err"))
    }

    /// Does the value (explicitly) exist in storage?
    pub fn contains_key<T: MerkleDB>(
        store: Arc<RwLock<State<T>>>,
        k1: &Key1,
        k2: &Key2,
    ) -> bool {
        store
            .read()
            .exists(Self::build_key_for(k1, k2).as_slice())
            .unwrap_or(false)
    }

    /// Load the value associated with the given key from the map.
    pub fn get<T: MerkleDB>(
        store: Arc<RwLock<State<T>>>,
        k1: &Key1,
        k2: &Key2,
    ) -> Option<Value> {
        let output = store
            .read()
            .get(Self::build_key_for(k1, k2).as_slice())
            .unwrap_or(None);
        if let Some(val) = output {
            serde_json::from_slice::<Value>(val.as_slice()).ok()
        } else {
            None
        }
    }

    /// Store a value to be associated with the given key from the map.
    pub fn insert<T: MerkleDB>(
        store: Arc<RwLock<State<T>>>,
        k1: &Key1,
        k2: &Key2,
        val: &Value,
    ) {
        let _ = serde_json::to_vec(val)
            .map(|v| store.write().set(Self::build_key_for(k1, k2).as_slice(), v));
    }

    /// Remove the value under a key.
    pub fn remove<T: MerkleDB>(store: Arc<RwLock<State<T>>>, k1: &Key1, k2: &Key2) {
        let _ = store.write().delete(Self::build_key_for(k1, k2).as_slice());
    }

    /// Remove all values under the first key.
    pub fn remove_prefix<T: MerkleDB>(store: Arc<RwLock<State<T>>>, k1: &Key1) {
        for (k2, _) in Self::iterate_prefix(store.clone(), k1).iter() {
            Self::remove(store.clone(), k1, k2);
        }
    }

    /// Iter over all value of the storage.
    pub fn iterate_prefix<T: MerkleDB>(
        store: Arc<RwLock<State<T>>>,
        k1: &Key1,
    ) -> Vec<(Key2, Value)> {
        let prefix_key: Vec<u8> =
            [Self::module_prefix(), Self::storage_prefix()].concat();
        let data_key1 = k1.to_string();
        let final_key =
            storage::store::Prefix::new(prefix_key.as_slice()).push(data_key1.as_ref());

        // Iterate db
        let mut kv_map = KVecMap::new();
        store.read().iterate(
            &final_key.begin(),
            &final_key.end(),
            IterOrder::Asc,
            &mut |(k, v)| -> bool {
                kv_map.insert(k, v);
                false
            },
        );
        // Iterate cache
        store.read().iterate_cache(final_key.as_ref(), &mut kv_map);

        let mut res = Vec::new();
        for (k, v) in kv_map {
            let key_str = String::from_utf8_lossy(k.as_slice()).to_string();
            let key_list: Vec<_> = key_str.split(DB_SEPARATOR).collect();

            let key = Self::parse_key_for(key_list);
            let raw_value = serde_json::from_slice::<Value>(v.as_slice()).ok();

            if let (Ok(k), Some(v)) = (key, raw_value) {
                res.push((k, v))
            }
        }
        res
    }
}
