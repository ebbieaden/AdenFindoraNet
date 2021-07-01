mod client;
mod genesis;
mod keeper;

use abci::*;
use evm::backend::Basic as Account;
use keeper::Keeper;
use primitive_types::{H160, H256, U256};
use primitives::crypto::secp256k1_ecdsa_recover;
use primitives::{
    context::Context,
    crypto::Address32,
    module::*,
    support::Get,
    transaction::{Executable, ValidateUnsigned},
};
use ruc::{eg, Result};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::marker::PhantomData;

pub const MODULE_NAME: &str = "ethereum";

pub struct EthereumModule<C> {
    name: String,
    keeper: Keeper,
    phantom: PhantomData<C>,
}

pub trait Config: app_evm::Config + Send + Sync {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Transact(ethereum::Transaction),
}

impl Executable for Action {
    type Origin = Address32;

    fn execute(self, _origin: Option<Self::Origin>, _ctx: Context) -> Result<()> {
        match self {
            Action::Transact(tx) => Ok(()),
        }
    }
}

impl<C: Config> EthereumModule<C> {
    pub fn new() -> Self {
        EthereumModule {
            name: MODULE_NAME.to_string(),
            keeper: Keeper::new(),
            phantom: Default::default(),
        }
    }
}

impl<C: Config> Default for EthereumModule<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Config> AppModuleBasic for EthereumModule<C> {
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

impl<C: Config> AppModuleGenesis for EthereumModule<C> {
    fn init_genesis(&self) {
        todo!()
    }

    fn export_genesis(&self) {
        todo!()
    }
}

impl<C: Config> AppModule for EthereumModule<C> {
    fn query_route(&self, path: Vec<&str>, req: &RequestQuery) -> ResponseQuery {
        ResponseQuery::new()
    }

    fn begin_block(&mut self, _req: &RequestBeginBlock) {
        todo!()
    }

    fn end_block(&mut self, _req: &RequestEndBlock) -> ResponseEndBlock {
        todo!()
    }
}

impl<C: Config> EthereumModule<C> {
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
}

impl<C: Config> ValidateUnsigned for EthereumModule<C> {
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
