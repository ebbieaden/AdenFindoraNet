mod basic;
mod client;
mod genesis;
mod impls;

use abci::{RequestQuery, ResponseQuery};
use fp_core::{
    account::{FinerTransfer, TransferToUTXO},
    context::Context,
    crypto::Address,
    ensure,
    module::AppModule,
    transaction::{ActionResult, Executable},
};
use fp_traits::account::{AccountAsset, FeeCalculator};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub const MODULE_NAME: &str = "account";

pub trait Config {
    type FeeCalculator: FeeCalculator;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transfer(FinerTransfer),
    TransferToUTXO(TransferToUTXO),
}

mod storage {
    use fp_core::{account::MintOutput, account::SmartAccount, crypto::Address};
    use fp_storage::*;

    // Store account information under all account addresses
    generate_storage!(Account, AccountStore => Map<Address, SmartAccount>);
    // Store MintOutputDefine
    generate_storage!(Account, MintOutputs => Value<Vec<MintOutput>>);
}

pub struct App<C> {
    phantom: PhantomData<C>,
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            phantom: Default::default(),
        }
    }
}

impl<C: Config> AppModule for App<C> {
    fn query_route(
        &self,
        ctx: Context,
        path: Vec<&str>,
        req: &RequestQuery,
    ) -> ResponseQuery {
        let mut resp = ResponseQuery::new();
        if path.len() != 1 {
            resp.code = 1;
            resp.log = String::from("account: invalid query path");
            return resp;
        }
        match path[0] {
            "nonce" => {
                let data = serde_json::from_slice::<Address>(req.data.as_slice());
                if data.is_err() {
                    resp.code = 1;
                    resp.log = String::from("account: query nonce with invalid params");
                    return resp;
                }
                let nonce = Self::nonce(&ctx, &data.unwrap());
                resp.value = serde_json::to_vec(&nonce).unwrap();
                resp
            }
            _ => resp,
        }
    }
}

impl<C: Config> AppModule for App<C> {
    fn query_route(
        &self,
        ctx: Context,
        path: Vec<&str>,
        req: &RequestQuery,
    ) -> ResponseQuery {
        let mut resp = ResponseQuery::new();
        if path.len() != 1 {
            resp.code = 1;
            resp.log = String::from("account: invalid query path");
            return resp;
        }
        match path[0] {
            "nonce" => {
                let data = serde_json::from_slice::<Address>(req.data.as_slice());
                if data.is_err() {
                    resp.code = 1;
                    resp.log = String::from("account: query nonce with invalid params");
                    return resp;
                }
                let nonce = Self::nonce(&ctx, &data.unwrap());
                resp.value = serde_json::to_vec(&nonce).unwrap();
                resp
            }
            _ => resp,
        }
    }
}

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        origin: Option<Self::Origin>,
        call: Self::Call,
        ctx: &Context,
    ) -> Result<ActionResult> {
        match call {
            Action::Transfer(action) => {
                if let Some(sender) = origin {
                    ensure!(action.nonce == Self::nonce(ctx, &sender), "InvalidNonce");
                    Self::inc_nonce(ctx, &sender)?;
                    Self::transfer(ctx, &sender, &action.to, action.amount)?;
                    Ok(ActionResult::default())
                } else {
                    Err(eg!("invalid transaction origin"))
                }
            }
            Action::TransferToUTXO(action) => {
                if let Some(sender) = origin {
                    ensure!(action.nonce == Self::nonce(ctx, &sender), "InvalidNonce");
                    Self::inc_nonce(ctx, &sender)?;
                    Self::transfer_to_utxo(ctx, sender, action.outputs)
                } else {
                    Err(eg!("invalid transaction origin"))
                }
            }
        }
    }
}
