mod basic;
mod client;
mod genesis;
mod impls;

use abci::{RequestEndBlock, ResponseEndBlock};
use fp_core::{
    context::Context,
    crypto::Address,
    ensure,
    macros::Get,
    module::AppModule,
    transaction::{ActionResult, Executable, ValidateUnsigned},
};
use fp_traits::evm::{DecimalsMapping, FeeCalculator};
use primitive_types::U256;
use ruc::{eg, Result, RucResult};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use storage::*;

pub trait Config: module_evm::Config {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transact(ethereum::Transaction),
}

pub mod storage {
    use ethereum::{Block, Receipt, Transaction};
    use fp_evm::TransactionStatus;
    use fp_storage::*;
    use primitive_types::{H256, U256};

    // Current building block's transactions and receipts.
    generate_storage!(Ethereum, Pending => Value<Vec<(Transaction, TransactionStatus, Receipt)>>);
    // The current Ethereum block.
    generate_storage!(Ethereum, CurrentBlock => Value<Option<Block>>);
    // The current Ethereum receipts.
    generate_storage!(Ethereum, CurrentReceipts => Value<Option<Vec<Receipt>>>);
    // The current transaction statuses.
    generate_storage!(Ethereum, CurrentTransactionStatuses => Value<Option<Vec<TransactionStatus>>>);
    // Mapping for block number and hashes.
    generate_storage!(Ethereum, BlockHash => Map<U256, H256>);
}

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            name: "ethereum".to_string(),
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
    fn end_block(
        &mut self,
        ctx: &mut Context,
        req: &RequestEndBlock,
    ) -> ResponseEndBlock {
        if Pending::exists(ctx.store.clone()) {
            let _ = ruc::info!(Self::store_block(ctx, U256::from(req.height)));
            Pending::delete(ctx.store.clone());
        }
        ResponseEndBlock::new()
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
        ensure!(origin.is_none(), "InvalidTransaction: IllegalOrigin");

        match call {
            Action::Transact(tx) => Self::do_transact(ctx, tx),
        }
    }
}

impl<C: Config> ValidateUnsigned for App<C> {
    type Call = Action;

    fn validate_unsigned(call: &Self::Call, ctx: &Context) -> Result<()> {
        let Action::Transact(transaction) = call;
        if let Some(chain_id) = transaction.signature.chain_id() {
            if chain_id != C::ChainId::get() {
                return Err(eg!("TransactionValidationError: InvalidChainId"));
            }
        }

        let origin = Self::recover_signer(&transaction)
            .ok_or_else(|| eg!("TransactionValidationError: InvalidSignature"))?;

        if transaction.gas_limit > C::BlockGasLimit::get() {
            return Err(eg!("TransactionValidationError: InvalidGasLimit"));
        }

        if transaction.gas_price < C::FeeCalculator::min_gas_price() {
            return Err(eg!("InvalidTransaction: Payment"));
        }

        let account_data = module_evm::App::<C>::account_basic(ctx, &origin);

        if transaction.nonce < account_data.nonce {
            return Err(eg!("InvalidTransaction: Outdated"));
        }

        let fee = transaction.gas_price.saturating_mul(transaction.gas_limit);
        let total_payment = transaction.value.saturating_add(fee);
        let total_payment = C::DecimalsMapping::into_native_token(total_payment);
        if account_data.balance < total_payment {
            return Err(eg!(format!(
                "InvalidTransaction: InsufficientBalance, expected:{}, actual:{}",
                total_payment, account_data.balance
            )));
        }

        Ok(())
    }
}
