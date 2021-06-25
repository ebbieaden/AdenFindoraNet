//! Smart address operation for transaction.

use crate::address::smart_address::SmartAddress;
use crate::address::store::SmartAddressStorage;
use crate::data_model::NoReplayToken;
use crate::data_model::XfrAddress;
use ruc::*;
use serde::{Deserialize, Serialize};
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey, XfrSignature};

/// Use this operation to bind more type of address.
///
/// This operation only support binded xfr_address is sender address.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BindAddressOp {
    pub data: Data,
    pub public: XfrPublicKey,
    pub signature: XfrSignature,
}

impl BindAddressOp {
    pub fn new(
        keypair: XfrKeyPair,
        smart_address: SmartAddress,
        nonce: NoReplayToken,
    ) -> Self {
        let data = Data::new(smart_address, nonce);
        let public = keypair.get_pk();
        let signature = keypair.sign(&data.to_bytes());
        BindAddressOp {
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
        let xfr_smart_address = XfrAddress {
            key: self.public.clone(),
        };
        let eth_smart_address = self.data.smart_address.clone();
        store
            .bind_xfr_and_sa(xfr_smart_address, eth_smart_address)
            .c(d!())?;
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
    // pub xfr_address: XfrAddress,
    pub smart_address: SmartAddress,
    pub nonce: NoReplayToken,
}

impl Data {
    pub fn new(
        // xfr_address: XfrAddress,
        smart_address: SmartAddress,
        nonce: NoReplayToken,
    ) -> Self {
        Data {
            // xfr_address,
            smart_address,
            nonce,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        pnk!(bincode::serialize(self))
    }
}
