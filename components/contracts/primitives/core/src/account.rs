use crate::crypto::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zei::xfr::sig::XfrPublicKey;
use zei::xfr::structs::AssetType;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartAccount {
    /// Account nonce.
    pub nonce: u64,
    /// Account balance(native asset).
    pub balance: u128,
    /// Balance which is reserved and may not be used.
    /// such as: staking deposit
    pub reserved: u128,
    /// Other crypto asset balances.
    pub assets: HashMap<AssetType, u128>,
}

/// Account balance convert to utxo balance.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MintOutput {
    pub asset: AssetType,
    pub amount: u64,
    pub target: XfrPublicKey,
}

/// Findora or Ethereum account address balance transfer to utxo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferToUTXO {
    pub nonce: u64,
    pub outputs: Vec<MintOutput>,
}

/// Findora native account address balance transfer to another account address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinerTransfer {
    pub nonce: u64,
    pub to: Address,
    pub amount: u128,
}
