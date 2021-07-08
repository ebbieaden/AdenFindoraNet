mod basic;
mod client;
mod genesis;
mod impls;

use fp_core::{
    context::Context, crypto::Address, module::AppModule, transaction::Executable,
};
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub trait Config {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transfer((Address, u128)),
}

mod storage {
    use fp_core::{account::SmartAccount, crypto::Address};
    use fp_storage::*;

    generate_storage!(Account, AccountStore => Map<Address, SmartAccount>);
}

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            name: "account".to_string(),
            phantom: Default::default(),
        }
    }
}

impl<C: Config> Default for App<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Config> AppModule for App<C> {}

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        origin: Option<Self::Origin>,
        call: Self::Call,
        ctx: &Context,
    ) -> Result<()> {
        match call {
            Action::Transfer((dest, balance)) => {
                if let Some(sender) = origin {
                    Self::do_transfer(ctx, &sender, &dest, balance)
                } else {
                    Err(eg!("invalid transaction origin"))
                }
            }
        }
    }
}
