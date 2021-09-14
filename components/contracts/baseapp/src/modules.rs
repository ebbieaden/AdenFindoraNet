use super::*;
use abci::{RequestBeginBlock, RequestEndBlock, RequestQuery, ResponseEndBlock};
use fp_core::{
    account::MintOutput,
    context::Context,
    crypto::Address,
    module::AppModule,
    transaction::{ActionResult, Applyable, Executable, ValidateUnsigned},
};
use ledger::{
    address::operation::check_convert_tx, data_model::Transaction as FindoraTransaction,
};
use ruc::*;

pub struct ModuleManager {
    // Ordered module list
    pub(crate) account_module: module_account::App<BaseApp>,
    pub(crate) ethereum_module: module_ethereum::App<BaseApp>,
    pub(crate) evm_module: module_evm::App<BaseApp>,
    pub(crate) template_module: module_template::App<BaseApp>,
}

impl ModuleManager {
    pub fn new() -> Self {
        ModuleManager {
            account_module: module_account::App::new(),
            ethereum_module: module_ethereum::App::new(),
            evm_module: module_evm::App::new(),
            template_module: module_template::App::new(),
        }
    }
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
        if module_name == module_account::MODULE_NAME {
            self.account_module.query_route(ctx, path, req)
        } else if module_name == module_ethereum::MODULE_NAME {
            self.ethereum_module.query_route(ctx, path, req)
        } else if module_name == module_evm::MODULE_NAME {
            self.evm_module.query_route(ctx, path, req)
        } else if module_name == module_template::MODULE_NAME {
            self.template_module.query_route(ctx, path, req)
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
        self.template_module.begin_block(ctx, req);
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
        let resp_template = self.template_module.end_block(ctx, req);
        if resp_template.validator_updates.len() > 0 {
            resp.set_validator_updates(resp_template.validator_updates);
        }
        resp
    }

    pub fn process_tx(
        &mut self,
        mut ctx: Context,
        tx: UncheckedTransaction,
    ) -> Result<ActionResult> {
        let checked = tx.clone().check()?;
        match tx.function.clone() {
            Action::Ethereum(action) => {
                // handle unsigned transaction, set specified module.
                let module_ethereum::Action::Transact(eth_tx) = action.clone();
                ctx.tx = serde_json::to_vec(&eth_tx)
                    .map_err(|e| eg!(format!("Serialize ethereum tx err: {}", e)))?;

                Self::dispatch::<module_ethereum::Action, module_ethereum::App<BaseApp>>(
                    &ctx, action, checked,
                )
            }
            _ => Self::dispatch::<Action, BaseApp>(&ctx, tx.function, checked),
        }
    }

    pub fn process_findora_tx(
        &mut self,
        ctx: &Context,
        tx: &FindoraTransaction,
    ) -> Result<()> {
        let (owner, assets) = check_convert_tx(tx)?;
        for (asset, amount) in assets.iter() {
            module_account::App::<BaseApp>::mint(
                ctx,
                &Address::from(owner.clone()),
                amount.clone().into(),
                asset.clone(),
            )?;
        }
        Ok(())
    }

    pub fn consume_mint(
        &mut self,
        ctx: &Context,
        size: usize,
    ) -> Result<Vec<MintOutput>> {
        module_account::App::<BaseApp>::consume_mint(ctx, size)
    }
}

impl ModuleManager {
    fn dispatch<Call, Module>(
        ctx: &Context,
        action: Call,
        tx: CheckedTransaction,
    ) -> Result<ActionResult>
    where
        Module: ValidateUnsigned<Call = Call>,
        Module: Executable<Origin = Address, Call = Call>,
    {
        let origin_tx = convert_unsigned_transaction::<Call>(action, tx);

        origin_tx.validate::<Module>(ctx)?;

        if RunTxMode::Deliver == ctx.run_mode {
            return origin_tx.apply::<Module>(ctx);
        }
        Ok(ActionResult::default())
    }
}
