use crate::hash::StorageHasher;
use crate::*;
use storage::db::IterOrder;
use storage::state::KVecMap;

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
    Key: Serialize + DeserializeOwned,
    Value: Serialize + DeserializeOwned,
{
    pub fn module_prefix() -> &'static [u8] {
        Prefix::module_prefix().as_bytes()
    }

    pub fn storage_prefix() -> &'static [u8] {
        Prefix::STORAGE_PREFIX.as_bytes()
    }

    /// Get the storage key used to fetch a value corresponding to a specific key.
    pub fn hashed_key_for(key: Key) -> Vec<u8> {
        let prefix_key: Vec<u8> =
            [Self::module_prefix(), Self::storage_prefix()].concat();
        let prefix_key_hashed = Hasher::hash(prefix_key.as_slice());
        let data_key = serde_json::to_vec(&key).unwrap_or(vec![]);
        let mut final_key =
            Vec::with_capacity(prefix_key_hashed.len() + data_key.as_slice().len());
        final_key.extend_from_slice(&prefix_key_hashed[..]);
        final_key.extend_from_slice(data_key.as_slice());
        final_key
    }

    /// Does the value (explicitly) exist in storage?
    pub fn contains_key(store: Arc<RwLock<Store>>, key: Key) -> bool {
        store
            .read()
            .exists(Self::hashed_key_for(key).as_ref())
            .unwrap_or(false)
    }

    /// Load the value associated with the given key from the map.
    pub fn get(store: Arc<RwLock<Store>>, key: Key) -> Option<Value> {
        let output = store
            .read()
            .get(Self::hashed_key_for(key).as_ref())
            .unwrap_or(None);
        if let Some(val) = output {
            serde_json::from_slice::<Value>(val.as_slice()).ok()
        } else {
            None
        }
    }

    /// Store a value to be associated with the given key from the map.
    pub fn insert(store: Arc<RwLock<Store>>, key: Key, val: Value) {
        let _ = serde_json::to_vec(&val)
            .map(|v| store.write().set(Self::hashed_key_for(key).as_ref(), v));
    }

    /// Remove the value under a key.
    pub fn remove(store: Arc<RwLock<Store>>, key: Key) {
        let _ = store.write().delete(Self::hashed_key_for(key).as_ref());
    }

    /// Iter over all value of the storage.
    pub fn iterate(store: Arc<RwLock<Store>>) -> Vec<(Key, Value)> {
        let prefix_key: Vec<u8> =
            [Self::module_prefix(), Self::storage_prefix()].concat();
        let prefix_key_hashed = &Hasher::hash(prefix_key.as_slice())[..];

        // Iterate db
        let mut kv_map = KVecMap::new();
        store.read().iterate(
            prefix_key_hashed,
            prefix_key_hashed,
            IterOrder::Asc,
            &mut |(k, v)| -> bool {
                kv_map.insert(k, v);
                println!("==========");
                false
            },
        );
        // Iterate cache
        store.read().iterate_cache(prefix_key_hashed, &mut kv_map);

        let mut res = Vec::new();
        for (k, v) in kv_map {
            let actual_key = k.clone().split_off(prefix_key_hashed.len());
            let raw_key = serde_json::from_slice::<Key>(actual_key.as_slice()).ok();
            let raw_value = serde_json::from_slice::<Value>(v.as_slice()).ok();
            if raw_key.is_some() && raw_value.is_some() {
                res.push((raw_key.unwrap(), raw_value.unwrap()))
            }
        }
        res
    }
}
