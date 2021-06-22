use ruc::Result;
use std::any::Any;

pub mod evm;

pub trait TxMsg {
    fn route_path(&self) -> String;

    /// Do a simple validation check that doesn't require access to any other information.
    fn validate_basic(&self) -> Result<()>;

    fn as_any(&self) -> &dyn Any;
}
