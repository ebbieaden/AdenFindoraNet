use crate::hash::StorageHasher;
use crate::*;
use std::str::FromStr;
use storage::db::{IterOrder, MerkleDB};
use storage::state::{KVecMap, State};

/// A type that allow to store value for given key. Allowing to insert/remove/iterate on values.
///
/// Each value is stored at:
/// ```nocompile
/// Sha256(Prefix::module_prefix() + Prefix::STORAGE_PREFIX)
///     ++ serialize(key)
/// ```
///
pub struct StorageMap<Prefix, Hasher, Key, Value>(
    core::marker::PhantomData<(Prefix, Hasher, Key, Value)>,
);

impl<Prefix, Hasher, Key, Value> StorageMap<Prefix, Hasher, Key, Value>
where
    Prefix: StorageInstance,
    Hasher: StorageHasher<Output = [u8; 32]>,
    Key: ToString + FromStr,
    Value: Serialize + DeserializeOwned,
{
    pub fn module_prefix() -> &'static [u8] {
        Prefix::module_prefix().as_bytes()
    }

    pub fn storage_prefix() -> &'static [u8] {
        Prefix::STORAGE_PREFIX.as_bytes()
    }

    /// Get the storage key used to fetch a value corresponding to a specific key.
    pub fn build_key_for(key: &Key) -> Vec<u8> {
        let prefix_key: Vec<u8> =
            [Self::module_prefix(), Self::storage_prefix()].concat();
        let data_key = key.to_string();

        let final_key = storage::store::Prefix::new(prefix_key.as_slice());
        final_key.push(data_key.as_ref()).as_ref().to_vec()
    }

    pub fn parse_key_for(key_list: Vec<&str>) -> std::result::Result<Key, ()> {
        let last_key = key_list.last().copied();
        if last_key.is_none() {
            return Err(());
        }
        let key = Key::from_str(last_key.unwrap());
        match key {
            Ok(k) => Ok(k),
            Err(_) => Err(()),
        }
    }

    /// Does the value (explicitly) exist in storage?
    pub fn contains_key<T: MerkleDB>(store: Arc<RwLock<State<T>>>, key: &Key) -> bool {
        store
            .read()
            .exists(Self::build_key_for(key).as_slice())
            .unwrap_or(false)
    }

    /// Read the length of the storage value without decoding the entire value under the
    /// given `key`.
    pub fn decode_len<T: MerkleDB>(
        store: Arc<RwLock<State<T>>>,
        key: &Key,
    ) -> Option<usize> {
        let output = store
            .read()
            .get(Self::build_key_for(key).as_slice())
            .unwrap_or(None);
        if let Some(val) = output {
            Some(val.len())
        } else {
            None
        }
    }

    /// Load the value associated with the given key from the map.
    pub fn get<T: MerkleDB>(store: Arc<RwLock<State<T>>>, key: &Key) -> Option<Value> {
        let output = store
            .read()
            .get(Self::build_key_for(key).as_slice())
            .unwrap_or(None);
        if let Some(val) = output {
            serde_json::from_slice::<Value>(val.as_slice()).ok()
        } else {
            None
        }
    }

    /// Store a value to be associated with the given key from the map.
    pub fn insert<T: MerkleDB>(store: Arc<RwLock<State<T>>>, key: &Key, val: &Value) {
        let _ = serde_json::to_vec(val)
            .map(|v| store.write().set(Self::build_key_for(key).as_slice(), v));
    }

    /// Remove the value under a key.
    pub fn remove<T: MerkleDB>(store: Arc<RwLock<State<T>>>, key: &Key) {
        let _ = store.write().delete(Self::build_key_for(key).as_slice());
    }

    /// Iter over all value of the storage.
    pub fn iterate<T: MerkleDB>(store: Arc<RwLock<State<T>>>) -> Vec<(Key, Value)> {
        let prefix_key: Vec<u8> =
            [Self::module_prefix(), Self::storage_prefix()].concat();
        let prefix: storage::store::Prefix =
            storage::store::Prefix::new(prefix_key.as_ref());

        // Iterate db
        // Iterate db
        let mut kv_map = KVecMap::new();
        store.read().iterate(
            &prefix.begin(),
            &prefix.end(),
            IterOrder::Asc,
            &mut |(k, v)| -> bool {
                kv_map.insert(k, v);
                false
            },
        );
        // Iterate cache
        store.read().iterate_cache(prefix.as_ref(), &mut kv_map);

        let mut res = Vec::new();
        for (k, v) in kv_map {
            let key_str = String::from_utf8_lossy(k.as_slice()).to_string();
            let key_list: Vec<_> = key_str.split(DB_SEPARATOR).collect();

            let key = Self::parse_key_for(key_list);
            let raw_value = serde_json::from_slice::<Value>(v.as_slice()).ok();

            if key.is_ok() && raw_value.is_some() {
                res.push((key.unwrap(), raw_value.unwrap()))
            }
        }
        res
    }
}
