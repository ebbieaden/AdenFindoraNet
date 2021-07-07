use super::store::SmartAddressStorage;
use crate::data_model::Operation;
use crate::data_model::Transaction;
use ruc::*;
use std::path::Path;

pub struct AddressBinder {
    storage: SmartAddressStorage,
}

impl AddressBinder {
    pub fn new(path: &Path) -> Result<Self> {
        Ok(AddressBinder {
            storage: SmartAddressStorage::new(path)?,
        })
    }

    pub fn get_storage(&self) -> &SmartAddressStorage {
        &self.storage
    }

    pub fn test() -> Result<Self> {
        Ok(AddressBinder {
            storage: SmartAddressStorage::new(&Path::new(
                "/tmp/findora-account-binder",
            ))?,
        })
    }

    pub fn check_tx(&self) -> Result<bool> {
        Ok(false)
    }

    pub fn deliver_tx(&self, tx: &Transaction) -> Result<()> {
        for op in tx.body.operations.iter() {
            match op {
                Operation::BindAddressOp(bind) => {
                    bind.apply_store(&self.storage).c(d!())?
                }
                Operation::UnbindAddressOp(unbind) => {
                    unbind.apply_store(&self.storage).c(d!())?
                }
                _ => {}
            }
        }
        Ok(())
    }
}
