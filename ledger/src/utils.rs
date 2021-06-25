use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct H160(pub [u8; 20]);

impl Default for H160 {
    fn default() -> Self {
        Self([0u8; 20])
    }
}
