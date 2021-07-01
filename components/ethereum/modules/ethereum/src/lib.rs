mod client;
mod genesis;

use abci::*;
use ethereum_types::{Bloom, BloomInput};
use evm::ExitReason;
use fp_core::{
    context::Context,
    crypto::{secp256k1_ecdsa_recover, Address},
    module::*,
    support::Get,
    transaction::{Executable, ValidateUnsigned},
};
use fp_evm::{Account, CallOrCreateInfo, TransactionStatus};
use module_evm::Runner;
use primitive_types::{H160, H256, U256};
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::marker::PhantomData;

pub const MODULE_NAME: &str = "ethereum";

pub struct App<C> {
    name: String,
    phantom: PhantomData<C>,
}

pub trait Config: module_evm::Config + Send + Sync {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transact(ethereum::Transaction),
}

impl<C: Config> App<C> {
    pub fn new() -> Self {
        App {
            name: MODULE_NAME.to_string(),
            phantom: Default::default(),
        }
    }
}

impl<C: Config> Default for App<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Config> AppModuleBasic for App<C> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn default_genesis(&self) -> Vec<u8> {
        todo!()
    }

    fn validate_genesis(&self) -> Result<()> {
        todo!()
    }

    fn register_rest_routes(&self) {
        todo!()
    }

    fn register_grpc_gateway_routes(&self) {
        todo!()
    }

    fn get_tx_cmd(&self) {
        todo!()
    }

    fn get_query_cmd(&self) {
        todo!()
    }
}

impl<C: Config> AppModuleGenesis for App<C> {
    fn init_genesis(&self) {
        todo!()
    }

    fn export_genesis(&self) {
        todo!()
    }
}

impl<C: Config> AppModule for App<C> {
    fn query_route(&self, _path: Vec<&str>, _req: &RequestQuery) -> ResponseQuery {
        ResponseQuery::new()
    }

    fn begin_block(&mut self, _req: &RequestBeginBlock) {
        todo!()
    }

    fn end_block(&mut self, _req: &RequestEndBlock) -> ResponseEndBlock {
        todo!()
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

    fn do_transact(transaction: ethereum::Transaction) -> Result<()> {
        // ensure!(
        // 	fp_consensus::find_pre_log(&frame_system::Module::<T>::digest()).is_err(),
        // 	Error::<T>::PreLogExists,
        // );

        let source = Self::recover_signer(&transaction)
            .ok_or_else(|| eg!("ExecuteTransaction: InvalidSignature"))?;

        let transaction_hash =
            H256::from_slice(Keccak256::digest(&rlp::encode(&transaction)).as_slice());
        // TODO
        // let transaction_index = Pending::get().len() as u32;
        let transaction_index = 0;

        let (to, contract_address, info) = Self::execute_transaction(
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

        // Pending::append((transaction, status, receipt));

        // Self::deposit_event(Event::Executed(
        //     source,
        //     contract_address.unwrap_or_default(),
        //     transaction_hash,
        //     reason,
        // ));
        // Ok(PostDispatchInfo {
        //     actual_weight: Some(T::GasWeightMapping::gas_to_weight(
        //         used_gas.unique_saturated_into(),
        //     )),
        //     pays_fee: Pays::No,
        // })
        // .into()
        Ok(())
    }

    /// Execute an Ethereum transaction.
    pub fn execute_transaction(
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
                let res = C::Runner::call(module_evm::Call {
                    source: from,
                    target,
                    input: input.clone(),
                    value,
                    gas_limit: gas_limit.low_u64(),
                    gas_price,
                    nonce,
                })?;

                Ok((Some(target), None, CallOrCreateInfo::Call(res)))
            }
            ethereum::TransactionAction::Create => {
                let res = C::Runner::create(module_evm::Create {
                    source: from,
                    init: input.clone(),
                    value,
                    gas_limit: gas_limit.low_u64(),
                    gas_price,
                    nonce,
                })?;

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

        // let mut builder = ValidTransactionBuilder::default()
        //         .and_provides((origin, transaction.nonce))
        //         .priority(if min_gas_price == U256::zero() {
        //             0
        //         } else {
        //             let target_gas = (transaction.gas_limit * transaction.gas_price) / min_gas_price;
        //             T::GasWeightMapping::gas_to_weight(target_gas.unique_saturated_into())
        //         });
        //
        // if transaction.nonce > account_data.nonce {
        //     if let Some(prev_nonce) = transaction.nonce.checked_sub(1.into()) {
        //         builder = builder.and_requires((origin, prev_nonce))
        //     }
        // }
        //
        // builder.build()
        Ok(())
    }
}

impl<C: Config> Executable for App<C> {
    type Origin = Address;
    type Call = Action;

    fn execute(
        _origin: Option<Self::Origin>,
        call: Self::Call,
        _ctx: Context,
    ) -> Result<()> {
        match call {
            Action::Transact(tx) => Self::do_transact(tx),
        }
    }
}
