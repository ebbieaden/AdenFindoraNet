//! Smart address operation for transaction.

use crate::data_model::{NoReplayToken};
use ruc::*;
use serde::{Deserialize, Serialize};
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey, XfrSignature};
use crate::data_model::Transaction;

/// Use this operation to transfer.
///
/// This operation only support binded xfr_address is sender address.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConvertAccount {
    pub data: Data,
    pub public: XfrPublicKey,
    pub signature: XfrSignature,
}

impl ConvertAccount {
    pub fn new(keypair: &XfrKeyPair, nonce: NoReplayToken) -> Self {
        let data = Data::new(nonce);
        let public = keypair.get_pk();
        let signature = keypair.sign(&data.to_bytes());
        Self {
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

    pub fn check_by_tx(&self, _tx: &Transaction) -> bool {
        true
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

/// The body of TranserToAccount.
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

// fn check_convert_tx_amount(tx: &Transaction) -> Result<u64> {
//     // let owners = Vec::new();
//     for _op in &tx.body.operations {
//
//     }
//     Ok(1)
// }
//
