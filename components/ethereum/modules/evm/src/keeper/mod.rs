use crate::message::*;
use ruc::Result;

// keeper of the evm store
pub struct Keeper {
    cfg: evm::Config,
}

impl Keeper {
    pub fn new() -> Keeper {
        Keeper {
            cfg: evm::Config::istanbul(),
        }
    }
}

pub trait EvmRunner {
    fn call(&self, params: &Call) -> Result<()>;

    fn create(&self, params: &Create) -> Result<()>;

    fn create2(&self, params: &Create2) -> Result<()>;
}

impl EvmRunner for Keeper {
    fn call(&self, _params: &Call) -> Result<()> {
        todo!()
    }

    fn create(&self, _params: &Create) -> Result<()> {
        todo!()
    }

    fn create2(&self, _params: &Create2) -> Result<()> {
        todo!()
    }
}
