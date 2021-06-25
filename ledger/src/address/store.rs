use super::SmartAddress;
use crate::data_model::XfrAddress;
// use crate::utils::H160;
use ruc::*;
use sled::{Db, IVec};
use std::path::Path;

/// Use persistent key-value map to store address mapping.
pub struct SmartAddressStorage {
    db: Db,
}

impl SmartAddressStorage {
    /// Create key-value store.
    pub fn new(path: &Path) -> Result<Self> {
        let db = sled::open(path).c(d!())?;
        Ok(SmartAddressStorage { db })
    }

    /// Get `SmartAddress` according `SmartAddress`.
    pub fn get(&self, key: &SmartAddress) -> Result<Option<SmartAddress>> {
        let key_sa = key.to_bytes().c(d!())?;
        if let Some(addr) = self.db.get(key_sa).c(d!())? {
            Ok(Some(SmartAddress::from_bytes(&addr).c(d!())?))
        } else {
            Ok(None)
        }
    }

    fn set(&self, key: &SmartAddress, value: &SmartAddress) -> Result<()> {
        let key_sa = key.to_bytes().c(d!())?;
        let value_sa = IVec::from(value.to_bytes().c(d!())?);
        self.db.insert(key_sa, value_sa).c(d!())?;
        Ok(())
    }

    pub fn del(&self, key: &SmartAddress) -> Result<()> {
        let key_sa = key.to_bytes().c(d!())?;
        self.db.remove(key_sa).c(d!())?;
        Ok(())
    }

    /// Use this function to bind xfr address and eth address.
    pub fn bind_xfr_and_sa(
        &self,
        xfr_address: XfrAddress,
        sa_address: SmartAddress,
    ) -> Result<()> {
        let sa_xfr = SmartAddress::Xfr(xfr_address);
        self.set(&sa_xfr, &sa_address).c(d!())?;
        self.set(&sa_address, &sa_xfr).c(d!())?;
        Ok(())
    }
}
