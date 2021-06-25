use primitives::{crypto::Address32, transaction::Executable};
use ruc::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transact(ethereum::Transaction),
}

impl Executable for Action {
    type Origin = Address32;
    // fn route_path(&self) -> String {
    //     crate::MODULE_NAME.to_string()
    // }

    fn execute(self, _origin: Option<Self::Origin>) -> Result<()> {
        Ok(())
    }

    // fn validate(&self) -> Result<()> {
    //     Ok(())
    // }
    //
    // fn as_any(&self) -> &dyn std::any::Any {
    //     self
    // }
}
