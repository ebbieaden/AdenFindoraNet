use crate::storage::*;
use crate::{App, Config};
use ethereum_types::{Bloom, BloomInput, H64};
use evm::ExitReason;
use fp_core::{context::Context, crypto::secp256k1_ecdsa_recover, macros::Get};
use fp_evm::{CallOrCreateInfo, TransactionStatus};
use module_evm::Runner;
use primitive_types::{H160, H256, U256};
use ruc::{eg, Result};
use sha3::{Digest, Keccak256};

impl<C: Config> App<C> {
    pub fn recover_signer(transaction: &ethereum::Transaction) -> Option<H160> {
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

    pub fn store_block(ctx: &mut Context, block_number: U256) -> Result<()> {
        let mut transactions = Vec::new();
        let mut statuses = Vec::new();
        let mut receipts = Vec::new();
        let mut logs_bloom = Bloom::default();
        let pending =
            Pending::get(ctx.store.clone()).ok_or(eg!("failed to get Pending"))?;
        for (transaction, status, receipt) in pending {
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

        CurrentBlock::put(ctx.store.clone(), Some(block));
        CurrentReceipts::put(ctx.store.clone(), Some(receipts));
        CurrentTransactionStatuses::put(ctx.store.clone(), Some(statuses));
        Ok(())
    }

    pub fn do_transact(ctx: Context, transaction: ethereum::Transaction) -> Result<()> {
        let source = Self::recover_signer(&transaction)
            .ok_or_else(|| eg!("ExecuteTransaction: InvalidSignature"))?;

        let transaction_hash =
            H256::from_slice(Keccak256::digest(&rlp::encode(&transaction)).as_slice());

        let mut pending =
            Pending::get(ctx.store.clone()).ok_or(eg!("failed to get Pending"))?;

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
        Pending::put(ctx.store, pending);

        Ok(())
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
