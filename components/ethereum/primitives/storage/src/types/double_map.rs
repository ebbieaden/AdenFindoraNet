use crate::hash::StorageHasher;
use crate::*;
use storage::db::IterOrder;
use storage::state::KVecMap;

/// A type that allow to store values for `(key1, key2)` couple. Similar to `StorageMap` but allow
/// to iterate and remove value associated to first key.
///
/// Each value is stored at:
/// ```nocompile
/// Sha256(Prefix::module_prefix() + Prefix::STORAGE_PREFIX)
///		++ serialize(key1)
///		++ serialize(key2)
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
    Key1: Serialize + DeserializeOwned,
    Key2: Serialize + DeserializeOwned,
    Value: Serialize + DeserializeOwned,
{
    pub fn module_prefix() -> &'static [u8] {
        Prefix::module_prefix().as_bytes()
    }

    pub fn storage_prefix() -> &'static [u8] {
        Prefix::STORAGE_PREFIX.as_bytes()
    }

    /// Get the storage key used to fetch a value corresponding to a specific key.
    pub fn hashed_key_for(k1: Key1, k2: Key2) -> Vec<u8> {
        let prefix_key: Vec<u8> =
            [Self::module_prefix(), Self::storage_prefix()].concat();
        let prefix_key_hashed = Hasher::hash(prefix_key.as_slice());
        let data_key1 = serde_json::to_vec(&k1).unwrap_or(vec![]);
        let data_key2 = serde_json::to_vec(&k2).unwrap_or(vec![]);
        let mut final_key = Vec::with_capacity(
            prefix_key_hashed.len()
                + data_key1.as_slice().len()
                + data_key2.as_slice().len(),
        );

        final_key.extend_from_slice(&prefix_key_hashed[..]);
        final_key.extend_from_slice(data_key1.as_slice());
        final_key.extend_from_slice(data_key2.as_slice());
        final_key
    }

    /// Does the value (explicitly) exist in storage?
    pub fn contains_key(store: Arc<RwLock<Store>>, k1: Key1, k2: Key2) -> bool {
        store
            .read()
            .exists(Self::hashed_key_for(k1, k2).as_ref())
            .unwrap_or(false)
    }

    /// Load the value associated with the given key from the map.
    pub fn get(store: Arc<RwLock<Store>>, k1: Key1, k2: Key2) -> Option<Value> {
        let output = store
            .read()
            .get(Self::hashed_key_for(k1, k2).as_ref())
            .unwrap_or(None);
        if let Some(val) = output {
            serde_json::from_slice::<Value>(val.as_slice()).ok()
        } else {
            None
        }
    }

    /// Store a value to be associated with the given key from the map.
    pub fn insert(store: Arc<RwLock<Store>>, k1: Key1, k2: Key2, val: Value) {
        let _ = serde_json::to_vec(&val)
            .map(|v| store.write().set(Self::hashed_key_for(k1, k2).as_ref(), v));
    }

    /// Remove the value under a key.
    pub fn remove(store: Arc<RwLock<Store>>, k1: Key1, k2: Key2) {
        let _ = store.write().delete(Self::hashed_key_for(k1, k2).as_ref());
    }

    // /// Remove all values under the first key.
    // pub fn remove_prefix(store: Arc<RwLock<Store>>, k1: Key1) {
    //     Self::iterate_prefix(store.clone(), k1)
    //         .iter()
    //         .map(|(k2, _)| {
    //             Self::remove(store.clone(), k1, k2);
    //         });
    // }

    /// Iter over all value of the storage.
    pub fn iterate_prefix(store: Arc<RwLock<Store>>, k1: Key1) -> Vec<(Key2, Value)> {
        let prefix_key: Vec<u8> =
            [Self::module_prefix(), Self::storage_prefix()].concat();
        let prefix_key_hashed = &Hasher::hash(prefix_key.as_slice())[..];
        let data_key1 = serde_json::to_vec(&k1).unwrap_or(vec![]);
        let mut final_key =
            Vec::with_capacity(prefix_key_hashed.len() + data_key1.as_slice().len());
        final_key.extend_from_slice(&prefix_key_hashed[..]);
        final_key.extend_from_slice(data_key1.as_slice());

        // Iterate db
        let mut kv_map = KVecMap::new();
        store.read().iterate(
            final_key.as_ref(),
            final_key.as_ref(),
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
            let actual_key = k.clone().split_off(final_key.len());
            let raw_key = serde_json::from_slice::<Key2>(actual_key.as_slice()).ok();
            let raw_value = serde_json::from_slice::<Value>(v.as_slice()).ok();
            if raw_key.is_some() && raw_value.is_some() {
                res.push((raw_key.unwrap(), raw_value.unwrap()))
            }
        }
        res
    }
}
