use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use zei::xfr::structs::AssetType;

#[serde_as]
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
    #[serde_as(as = "Vec<(_, _)>")]
    pub assets: HashMap<AssetType, u128>,
}
