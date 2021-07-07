use super::*;
use abci::{RequestBeginBlock, RequestEndBlock, RequestQuery, ResponseEndBlock};
use fp_core::{
    context::Context,
    crypto::Address,
    module::{AppModule, AppModuleBasic},
    transaction::{Applyable, Executable, ValidateUnsigned},
};
use ruc::Result;

#[derive(Default)]
pub struct ModuleManager {
    // Ordered module list
    account_module: module_account::App<BaseApp>,
    ethereum_module: module_ethereum::App<BaseApp>,
    evm_module: module_evm::App<BaseApp>,
}

impl ModuleManager {
    pub fn query(
        &self,
        ctx: Context,
        mut path: Vec<&str>,
        req: &RequestQuery,
    ) -> abci::ResponseQuery {
        let mut resp = abci::ResponseQuery::new();
        if 0 == path.len() {
            resp.set_code(1);
            resp.set_log("Invalid custom query path without module route!".to_string());
            return resp;
        }

        // Note: adding new modules may need to be updated.
        let module_name = path.remove(0);
        if module_name == self.account_module.name().as_str() {
            self.account_module.query_route(ctx, path, req)
        } else if module_name == self.ethereum_module.name().as_str() {
            self.ethereum_module.query_route(ctx, path, req)
        } else if module_name == self.evm_module.name().as_str() {
            self.evm_module.query_route(ctx, path, req)
        } else {
            resp.set_code(1);
            resp.set_log(format!("Invalid query module route: {}!", module_name));
            resp
        }
    }

    pub fn begin_block(&mut self, ctx: &mut Context, req: &RequestBeginBlock) {
        // Note: adding new modules need to be updated.
        self.account_module.begin_block(ctx, req);
        self.ethereum_module.begin_block(ctx, req);
        self.evm_module.begin_block(ctx, req);
    }

    pub fn end_block(
        &mut self,
        ctx: &mut Context,
        req: &RequestEndBlock,
    ) -> ResponseEndBlock {
        let mut resp = ResponseEndBlock::new();
        // Note: adding new modules need to be updated.
        let resp_account = self.account_module.end_block(ctx, req);
        if resp_account.validator_updates.len() > 0 {
            resp.set_validator_updates(resp_account.validator_updates);
        }
        let resp_eth = self.ethereum_module.end_block(ctx, req);
        if resp_eth.validator_updates.len() > 0 {
            resp.set_validator_updates(resp_eth.validator_updates);
        }
        let resp_evm = self.evm_module.end_block(ctx, req);
        if resp_evm.validator_updates.len() > 0 {
            resp.set_validator_updates(resp_evm.validator_updates);
        }
        resp
    }

    pub fn process_tx(
        &mut self,
        ctx: &Context,
        mode: RunTxMode,
        tx: UncheckedTransaction,
    ) -> Result<()> {
        // TODO check gas if deliver_tx

        let checked = tx.clone().check()?;
        // add match field if tx is unsigned transaction
        match tx.function {
            Action::Ethereum(action) => Self::dispatch::<
                module_ethereum::Action,
                module_ethereum::App<BaseApp>,
            >(ctx, mode, action, checked),
            _ => Self::dispatch::<Action, BaseApp>(ctx, mode, tx.function, checked),
        }
    }
}

// support functions
impl ModuleManager {
    fn dispatch<Call, Module>(
        ctx: &Context,
        mode: RunTxMode,
        action: Call,
        tx: CheckedTransaction,
    ) -> Result<()>
    where
        Module: ValidateUnsigned<Call = Call>,
        Module: Executable<Origin = Address, Call = Call>,
    {
        let origin_tx = convert_unsigned_transaction::<Call>(action, tx);

        origin_tx.validate::<Module>(ctx)?;

        if mode == RunTxMode::Deliver {
            origin_tx.apply::<Module>(ctx)?;

            ctx.store.write().commit_session();
        }
        Ok(())
    }
}
