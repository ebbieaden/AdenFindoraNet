//! Smart address operation for transaction.

use crate::address::smart_address::SmartAddress;
use crate::address::store::SmartAddressStorage;
use crate::data_model::{NoReplayToken, XfrAddress};
use ruc::*;
use serde::{Deserialize, Serialize};
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey, XfrSignature};

/// Use this operation to bind more type of address.
///
/// This operation only support binded xfr_address is sender address.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnbindAddressOp {
    pub data: Data,
    pub public: XfrPublicKey,
    pub signature: XfrSignature,
}

impl UnbindAddressOp {
    pub fn new(keypair: &XfrKeyPair, nonce: NoReplayToken) -> Self {
        let data = Data::new(nonce);
        let public = keypair.get_pk();
        let signature = keypair.sign(&data.to_bytes());
        UnbindAddressOp {
            data,
            public,
            signature,
        }
    }

    pub fn verify(&self) -> Result<()> {
        self.public
            .verify(&self.data.to_bytes(), &self.signature)
            .c(d!())
    }

    pub fn apply_store(&self, store: &SmartAddressStorage) -> Result<()> {
        let xfr_address = SmartAddress::Xfr(XfrAddress {
            key: self.public.clone(),
        });
        let sa_address = store.get(&xfr_address).c(d!())?;
        store.del(&xfr_address).c(d!())?;
        if let Some(addr) = sa_address {
            store.del(&addr).c(d!())?;
        }
        Ok(())
    }

    pub fn set_nonce(&mut self, nonce: NoReplayToken) {
        self.data.nonce = nonce;
    }

    pub fn get_nonce(&self) -> NoReplayToken {
        self.data.nonce
    }

    pub fn get_related_address(&self) -> XfrPublicKey {
        self.public
    }
}

/// The body of BindAddressOp.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub nonce: NoReplayToken,
}

impl Data {
    pub fn new(nonce: NoReplayToken) -> Self {
        Data { nonce }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        pnk!(bincode::serialize(self))
    }
}
