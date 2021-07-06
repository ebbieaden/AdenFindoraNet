pub use crate::context::*;
pub use named_type::NamedType;
pub use named_type_derive::*;
pub use ruc::{d, Result, RucResult};
pub use std::ops::{Deref, DerefMut};

/// Wrapper for access storage and deref tuple structs
#[macro_export]
macro_rules! storage_wrapper {
    ($name:ty, $type:ty) => {
        impl From<$type> for $name {
            fn from(v: $type) -> Self {
                Self(v)
            }
        }

        impl $crate::storage::Deref for $name {
            type Target = $type;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl $crate::storage::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl $name {
            pub fn get(store: Arc<RwLock<Store>>) -> Result<Self> {
                let output = store.read().get(Self::short_type_name().as_bytes())?;
                if let Some(val) = output {
                    Ok(serde_json::from_slice::<Self>(val.as_slice()).c(d!())?)
                } else {
                    Ok(Default::default())
                }
            }

            pub fn set(store: Arc<RwLock<Store>>, pending: &Self) -> Result<()> {
                let val = serde_json::to_vec(pending).c(d!())?;
                Ok(store.write().set(Self::short_type_name().as_bytes(), val))
            }
        }
    };
}
