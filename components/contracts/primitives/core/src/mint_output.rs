use serde::{Deserialize, Serialize};
use zei::xfr::sig::XfrPublicKey;
use zei::xfr::structs::AssetType;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MintOutput {
    pub asset: AssetType,
    pub amount: u64,
    pub target: XfrPublicKey,
}
