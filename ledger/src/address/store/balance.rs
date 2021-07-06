use cryptohash::sha256::Digest;
use ruc::*;
use serde::{Deserialize, Serialize};
use sparse_merkle_tree::{self, Key, SmtMap256};
use std::fs;
use std::path::{Path, PathBuf};
use zei::xfr::sig::XfrPublicKey;
use crate::data_model::Transaction;
use crate::address::operation::check_convert_tx_amount;

#[derive(Serialize, Deserialize)]
pub struct Balance {
    pub amount: u64,
    pub nonce: u64,
}

impl Default for Balance {
    fn default() -> Self {
        Balance {
            amount: 0,
            nonce: 0,
        }
    }
}

pub struct BalanceStore {
    db: SmtMap256<Vec<u8>>,
    path: PathBuf,
}

impl BalanceStore {
    pub fn test() -> Result<Self> {
        Self::new(Path::new("/tmp/findora-balance-store"))
    }

    fn open(path: &Path) -> Result<SmtMap256<Vec<u8>>> {
        let contents = fs::read(path).c(d!())?;

        bincode::deserialize(&contents).c(d!())
    }

    /// Create key-value store.
    pub fn new(path: &Path) -> Result<Self> {
        if path.exists() {
            Ok(BalanceStore {
                db: Self::open(path)?,
                path: path.to_path_buf(),
            })
        } else {
            Ok(BalanceStore {
                db: SmtMap256::new(),
                path: path.to_path_buf(),
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        fs::write(&self.path, bincode::serialize(&self.db).c(d!())?).c(d!())
    }

    /// Get balance by xfraddress.
    pub fn get(&self, address: &XfrPublicKey) -> Result<Option<Balance>> {
        let key = Key::hash(address.as_bytes());
        match self.db.get(&key) {
            Some(result) => Ok(bincode::deserialize(result).c(d!())?),
            None => Ok(None)
        }
    }

    pub fn root_hash(&self) -> &Digest {
        self.db.merkle_root()
    }

    pub fn set(&mut self, address: &XfrPublicKey, balance: &Balance) -> Result<()> {
        let key = Key::hash(address.as_bytes());
        self.db
            .set(&key, Some(bincode::serialize(balance).c(d!())?))
            .c(d!())?;
        Ok(())
    }

    pub fn check_tx(&self, tx: &Transaction) -> bool {
        check_convert_tx_amount(tx).is_ok()
    }

    pub fn deliver_tx(&mut self, tx: &Transaction) -> Result<()> {
        let (amount, owner) = check_convert_tx_amount(tx)?;

        let mut balance = self.get(&owner)?.unwrap_or(Balance::default());

        balance.nonce += 1;
        balance.amount += amount;

        self.set(&owner, &balance)?;

        self.save()
    }
}
