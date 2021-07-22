use crate::hash::StorageHasher;
use crate::*;
use storage::db::MerkleDB;
use storage::state::State;

/// A type that allow to store a value.
///
/// Each value is stored at:
/// ```nocompile
/// Sha256(Prefix::module_name() + Prefix::STORAGE_PREFIX)
/// ```
///
pub struct StorageValue<Prefix, Hasher, Value>(
    core::marker::PhantomData<(Prefix, Hasher, Value)>,
);

impl<Prefix, Hasher, Value> StorageValue<Prefix, Hasher, Value>
where
    Prefix: StorageInstance,
    Hasher: StorageHasher<Output = [u8; 32]>,
    Value: Serialize + DeserializeOwned,
{
    pub fn module_prefix() -> &'static [u8] {
        Prefix::module_prefix().as_bytes()
    }

    pub fn storage_prefix() -> &'static [u8] {
        Prefix::STORAGE_PREFIX.as_bytes()
    }

    /// Get the storage key.
    pub fn hashed_key() -> [u8; 32] {
        let raw_key: Vec<u8> = [Self::module_prefix(), Self::storage_prefix()].concat();
        Hasher::hash(raw_key.as_slice())
    }

    /// Does the value (explicitly) exist in storage?
    pub fn exists<T: MerkleDB>(store: Arc<RwLock<State<T>>>) -> bool {
        store
            .read()
            .exists(Self::hashed_key().as_ref())
            .unwrap_or(false)
    }

    /// Load the value from the provided storage instance.
    pub fn get<T: MerkleDB>(store: Arc<RwLock<State<T>>>) -> Option<Value> {
        let output = store
            .read()
            .get(Self::hashed_key().as_ref())
            .unwrap_or(None);
        if let Some(val) = output {
            serde_json::from_slice::<Value>(val.as_slice()).ok()
        } else {
            None
        }
    }

    /// Store a value under this hashed key into the provided storage instance.
    pub fn put<T: MerkleDB>(store: Arc<RwLock<State<T>>>, val: Value) {
        let _ = serde_json::to_vec(&val)
            .map(|v| store.write().set(Self::hashed_key().as_ref(), v));
    }

    /// Take a value from storage, removing it afterwards.
    pub fn delete<T: MerkleDB>(store: Arc<RwLock<State<T>>>) {
        let _ = store.write().delete(Self::hashed_key().as_ref());
    }
}
