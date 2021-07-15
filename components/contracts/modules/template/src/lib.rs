mod basic;
mod genesis;

pub use crate::storage::*;
use abci::{RequestEndBlock, RequestQuery, ResponseEndBlock, ResponseQuery};
use fp_core::{
    context::Context,
    crypto::Address,
    module::AppModule,
    transaction::{ActionResult, Executable},
};
use ruc::Result;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub trait Config {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    SetValue(u64),
}

mod storage {
    use fp_storage::*;

    generate_storage!(Template, ValueStore => Value<u64>);
}

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            name: "template".to_string(),
            phantom: Default::default(),
        }
    }
}

impl<C: Config> Default for App<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Config> AppModule for App<C> {
    fn query_route(
        &self,
        ctx: Context,
        path: Vec<&str>,
        _req: &RequestQuery,
    ) -> ResponseQuery {
        let mut resp = ResponseQuery::new();
        if path.len() != 1 {
            resp.code = 1;
            resp.log = String::from("template: invalid query path");
            return resp;
        }

        let value = ValueStore::get(ctx.store.clone()).unwrap_or_default();
        resp.value = serde_json::to_vec(&value).unwrap_or_default();
        resp
    }

    fn end_block(
        &mut self,
        _ctx: &mut Context,
        _req: &RequestEndBlock,
    ) -> ResponseEndBlock {
        ResponseEndBlock::new()
    }
}

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        _origin: Option<Self::Origin>,
        call: Self::Call,
        ctx: &Context,
    ) -> Result<ActionResult> {
        match call {
            Action::SetValue(v) => {
                ValueStore::put(ctx.store.clone(), v);
                let mut ar = ActionResult::default();
                ar.data = v.to_be_bytes().to_vec();
                Ok(ar)
            }
        }
    }
}
