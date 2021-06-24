use primitives::transaction::TxMsg;
use ruc::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Transact(ethereum::Transaction),
}

impl TxMsg for Message {
    fn route_path(&self) -> String {
        crate::MODULE_NAME.to_string()
    }

    fn execute(&self) -> Result<()> {
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
