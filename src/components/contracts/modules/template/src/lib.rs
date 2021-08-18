mod basic;
mod genesis;

pub use crate::storage::*;
use fp_core::{
    context::Context,
    module::AppModule,
    transaction::{ActionResult, Executable},
};
use fp_types::{actions::template::Action, crypto::Address};
use ruc::Result;
use std::marker::PhantomData;
use tm_protos::abci::{RequestQuery, ResponseQuery};

pub const MODULE_NAME: &str = "template";

pub trait Config {}

mod storage {
    use fp_storage::*;

    generate_storage!(Template, ValueStore => Value<u64>);
}

pub struct App<C> {
    phantom: PhantomData<C>,
}

impl<C: Config> Default for App<C> {
    fn default() -> Self {
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
        _req: &RequestQuery,
    ) -> ResponseQuery {
        let mut resp = ResponseQuery::default();
        if path.len() != 1 {
            resp.code = 1;
            resp.log = String::from("template: invalid query path");
            return resp;
        }

        let value = ValueStore::get(ctx.store).unwrap_or_default();
        resp.value = serde_json::to_vec(&value).unwrap_or_default();
        resp
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
                Ok(ActionResult {
                    data: v.to_be_bytes().to_vec(),
                    ..Default::default()
                })
            }
        }
    }
}
