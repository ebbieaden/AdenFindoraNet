mod basic;
mod client;
mod genesis;
mod storage;

use crate::storage::*;
use abci::{RequestEndBlock, RequestQuery, ResponseEndBlock, ResponseQuery};
use ethereum_types::{Bloom, BloomInput, H64};
use evm::ExitReason;
use fp_core::{
    context::Context,
    crypto::{secp256k1_ecdsa_recover, Address},
    ensure,
    macros::Get,
    module::AppModule,
    transaction::{Executable, ValidateUnsigned},
};
use fp_evm::{Account, CallOrCreateInfo, TransactionStatus};
use module_evm::Runner;
use primitive_types::{H160, H256, U256};
use ruc::{eg, Result, RucResult};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::marker::PhantomData;

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

pub trait Config: module_evm::Config {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transact(ethereum::Transaction),
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
    fn query_route(
        &self,
        _ctx: Context,
        _path: Vec<&str>,
        _req: &RequestQuery,
    ) -> ResponseQuery {
        ResponseQuery::new()
    }

    fn end_block(
        &mut self,
        ctx: &mut Context,
        req: &RequestEndBlock,
    ) -> ResponseEndBlock {
        let _ = ruc::info!(Self::store_block(ctx, U256::from(req.height)));
        ResponseEndBlock::new()
    }
}

impl<C: Config> App<C> {
    fn recover_signer(transaction: &ethereum::Transaction) -> Option<H160> {
        let mut sig = [0u8; 65];
        let mut msg = [0u8; 32];
        sig[0..32].copy_from_slice(&transaction.signature.r()[..]);
        sig[32..64].copy_from_slice(&transaction.signature.s()[..]);
        sig[64] = transaction.signature.standard_v();
        msg.copy_from_slice(
            &ethereum::TransactionMessage::from(transaction.clone()).hash()[..],
        );

        let pubkey = secp256k1_ecdsa_recover(&sig, &msg).ok()?;
        Some(H160::from(H256::from_slice(
            Keccak256::digest(&pubkey).as_slice(),
        )))
    }

    fn store_block(ctx: &mut Context, block_number: U256) -> Result<()> {
        let mut transactions = Vec::new();
        let mut statuses = Vec::new();
        let mut receipts = Vec::new();
        let mut logs_bloom = Bloom::default();
        let pending = Pending::get(ctx.store.clone())?;
        for (transaction, status, receipt) in (*pending).clone() {
            transactions.push(transaction);
            statuses.push(status);
            receipts.push(receipt.clone());
            Self::logs_bloom(receipt.logs.clone(), &mut logs_bloom);
        }

        let ommers = Vec::<ethereum::Header>::new();
        let partial_header = ethereum::PartialHeader {
            // parent_hash: Self::current_block_hash().unwrap_or_default(),
            parent_hash: H256::default(),
            // TODO find block author
            beneficiary: H160::default(),
            // TODO: figure out if there's better way to get a sort-of-valid state root.
            state_root: H256::default(),
            // TODO: check receipts hash.
            receipts_root: H256::from_slice(
                Keccak256::digest(&rlp::encode_list(&receipts)[..]).as_slice(),
            ),
            logs_bloom,
            difficulty: U256::zero(),
            number: block_number,
            gas_limit: C::BlockGasLimit::get(),
            gas_used: receipts
                .clone()
                .into_iter()
                .fold(U256::zero(), |acc, r| acc + r.used_gas),
            timestamp: ctx.block_time().get_seconds() as u64,
            extra_data: Vec::new(),
            mix_hash: H256::default(),
            nonce: H64::default(),
        };
        let mut block =
            ethereum::Block::new(partial_header, transactions.clone(), ommers);
        // TODO cache root hash?
        block.header.state_root =
            H256::from_slice(ctx.store.read().root_hash().as_slice());

        CurrentBlock::set(ctx.store.clone(), &Some(block).into())?;
        CurrentReceipts::set(ctx.store.clone(), &Some(receipts).into())?;
        CurrentTransactionStatuses::set(ctx.store.clone(), &Some(statuses).into())?;
        Ok(())
    }

    fn do_transact(ctx: Context, transaction: ethereum::Transaction) -> Result<()> {
        let source = Self::recover_signer(&transaction)
            .ok_or_else(|| eg!("ExecuteTransaction: InvalidSignature"))?;

        let transaction_hash =
            H256::from_slice(Keccak256::digest(&rlp::encode(&transaction)).as_slice());

        let mut pending = Pending::get(ctx.store.clone())?;

        // Note: the index is not the transaction index in the real block.
        let transaction_index = pending.len() as u32;

        let (to, contract_address, info) = Self::execute_transaction(
            &ctx,
            source,
            transaction.input.clone(),
            transaction.value,
            transaction.gas_limit,
            Some(transaction.gas_price),
            Some(transaction.nonce),
            transaction.action,
        )?;

        let (reason, status, used_gas) = match info {
            CallOrCreateInfo::Call(info) => (
                info.exit_reason,
                TransactionStatus {
                    transaction_hash,
                    transaction_index,
                    from: source,
                    to,
                    contract_address: None,
                    logs: info.logs.clone(),
                    logs_bloom: {
                        let mut bloom: Bloom = Bloom::default();
                        Self::logs_bloom(info.logs, &mut bloom);
                        bloom
                    },
                },
                info.used_gas,
            ),
            CallOrCreateInfo::Create(info) => (
                info.exit_reason,
                TransactionStatus {
                    transaction_hash,
                    transaction_index,
                    from: source,
                    to,
                    contract_address: Some(info.value),
                    logs: info.logs.clone(),
                    logs_bloom: {
                        let mut bloom: Bloom = Bloom::default();
                        Self::logs_bloom(info.logs, &mut bloom);
                        bloom
                    },
                },
                info.used_gas,
            ),
        };

        let receipt = ethereum::Receipt {
            state_root: match reason {
                ExitReason::Succeed(_) => H256::from_low_u64_be(1),
                ExitReason::Error(_) => H256::from_low_u64_le(0),
                ExitReason::Revert(_) => H256::from_low_u64_le(0),
                ExitReason::Fatal(_) => H256::from_low_u64_le(0),
            },
            used_gas,
            logs_bloom: status.clone().logs_bloom,
            logs: status.clone().logs,
        };

        pending.push((transaction, status, receipt));
        Pending::set(ctx.store, &pending)

        // TODO maybe events
    }

    /// Execute an Ethereum transaction.
    pub fn execute_transaction(
        ctx: &Context,
        from: H160,
        input: Vec<u8>,
        value: U256,
        gas_limit: U256,
        gas_price: Option<U256>,
        nonce: Option<U256>,
        action: ethereum::TransactionAction,
    ) -> Result<(Option<H160>, Option<H160>, CallOrCreateInfo)> {
        match action {
            ethereum::TransactionAction::Call(target) => {
                let res = C::Runner::call(
                    ctx,
                    module_evm::Call {
                        source: from,
                        target,
                        input: input.clone(),
                        value,
                        gas_limit: gas_limit.low_u64(),
                        gas_price,
                        nonce,
                    },
                )?;

                Ok((Some(target), None, CallOrCreateInfo::Call(res)))
            }
            ethereum::TransactionAction::Create => {
                let res = C::Runner::create(
                    ctx,
                    module_evm::Create {
                        source: from,
                        init: input.clone(),
                        value,
                        gas_limit: gas_limit.low_u64(),
                        gas_price,
                        nonce,
                    },
                )?;

                Ok((None, Some(res.value), CallOrCreateInfo::Create(res)))
            }
        }
    }

    fn logs_bloom(logs: Vec<ethereum::Log>, bloom: &mut Bloom) {
        for log in logs {
            bloom.accrue(BloomInput::Raw(&log.address[..]));
            for topic in log.topics {
                bloom.accrue(BloomInput::Raw(&topic[..]));
            }
        }
    }
}

impl<C: Config> ValidateUnsigned for App<C> {
    type Call = Action;

    fn validate_unsigned(call: &Self::Call, _ctx: Context) -> Result<()> {
        let Action::Transact(transaction) = call;
        if let Some(chain_id) = transaction.signature.chain_id() {
            if chain_id != C::ChainId::get() {
                return Err(eg!("TransactionValidationError: InvalidChainId"));
            }
        }

        let origin = Self::recover_signer(&transaction)
            .ok_or_else(|| eg!("TransactionValidationError: InvalidSignature"))?;

        if transaction.gas_limit >= C::BlockGasLimit::get() {
            return Err(eg!("TransactionValidationError: InvalidGasLimit"));
        }

        // TODO
        let account_data: Account = Default::default();
        // let account_data = pallet_evm::Module::<T>::account_basic(&origin);
        //
        if transaction.nonce < account_data.nonce {
            return Err(eg!("InvalidTransaction: Outdated"));
        }

        let fee = transaction.gas_price.saturating_mul(transaction.gas_limit);
        let total_payment = transaction.value.saturating_add(fee);
        if account_data.balance < total_payment {
            return Err(eg!("InvalidTransaction: InsufficientBalance"));
        }

        // TODO
        // let min_gas_price = T::FeeCalculator::min_gas_price();
        let min_gas_price = U256::zero();

        if transaction.gas_price < min_gas_price {
            return Err(eg!("InvalidTransaction: Payment"));
        }

        Ok(())
    }
}

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        origin: Option<Self::Origin>,
        call: Self::Call,
        ctx: Context,
    ) -> Result<()> {
        ensure!(origin.is_none(), "InvalidTransaction: IllegalOrigin");

        match call {
            Action::Transact(tx) => Self::do_transact(ctx, tx),
        }
    }
}
