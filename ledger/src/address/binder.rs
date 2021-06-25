use super::store::SmartAddressStorage;
use ruc::*;

pub struct AddressBinder {
    storage: SmartAddressStorage,
}

impl AddressBinder {
    pub fn new(path: &str) -> Result<Self> {
        Ok(AddressBinder {
            storage: SmartAddressStorage::new(path)?,
        })
    }

    pub fn check_tx(&self) -> Result<bool> {
        Ok(false)
    }

    pub fn deliver_tx(&self) -> Result<()> {
        Ok(())
    }
}

