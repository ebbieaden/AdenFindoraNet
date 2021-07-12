use crate::forward::*;
use crate::internal_err;
use baseapp::{BaseApp, ChainId, UncheckedTransaction};
use ethereum_types::{H160, H256, H64, U256, U64};
use fp_rpc_core::types::{
    BlockNumber, Bytes, CallRequest, Filter, FilterChanges, Index, Log, PeerCount,
    Receipt, RichBlock, SyncStatus, Transaction, TransactionRequest, Work,
};
use fp_rpc_core::{EthApi, EthFilterApi, NetApi, Web3Api};
use fp_traits::evm::{AddressMapping, EthereumAddressMapping};
use fp_utils::ethereum::{sign_transaction_message, KeyPair};
use jsonrpc_core::{futures::future, BoxFuture, Result};
use parking_lot::RwLock;
use sha3::{Digest, Keccak256};
use std::sync::Arc;

pub struct EthApiImpl {
    account_base_app: Arc<RwLock<BaseApp>>,
    signers: Vec<KeyPair>,
    forwarder: TendermintForward,
}

impl EthApiImpl {
    pub fn new(
        url: String,
        account_base_app: Arc<RwLock<BaseApp>>,
        signers: Vec<KeyPair>,
    ) -> Self {
        Self {
            account_base_app,
            signers,
            forwarder: TendermintForward::new(url),
        }
    }
}

impl EthApi for EthApiImpl {
    fn protocol_version(&self) -> Result<u64> {
        Ok(1)
    }

    fn hashrate(&self) -> Result<U256> {
        Ok(U256::zero())
    }

    fn chain_id(&self) -> Result<Option<U64>> {
        Ok(Some(ChainId::get().into()))
    }

    fn accounts(&self) -> Result<Vec<H160>> {
        let mut accounts = Vec::new();
        for signer in self.signers.iter() {
            accounts.push(signer.address.clone());
        }
        Ok(accounts)
    }

    fn balance(&self, address: H160, number: Option<BlockNumber>) -> Result<U256> {
        let ctx = if let Some(BlockNumber::Pending) = number {
            Some(self.account_base_app.read().check_state.clone())
        } else {
            None
        };

        let account_id = EthereumAddressMapping::into_account_id(address);
        let sa = self
            .account_base_app
            .read()
            .account_of(&account_id, ctx)
            .map_err(|e| internal_err(e))?;
        Ok(U256::from(sa.balance))
    }

    fn send_transaction(&self, request: TransactionRequest) -> BoxFuture<H256> {
        let from = match request.from {
            Some(from) => from,
            None => {
                let accounts = match self.accounts() {
                    Ok(accounts) => accounts,
                    Err(e) => return Box::new(future::result(Err(e))),
                };

                match accounts.get(0) {
                    Some(account) => account.clone(),
                    None => {
                        return Box::new(future::result(Err(internal_err(
                            "no signer available",
                        ))));
                    }
                }
            }
        };

        let nonce = match request.nonce {
            Some(nonce) => nonce,
            None => match self.transaction_count(from, None) {
                Ok(nonce) => nonce,
                Err(e) => return Box::new(future::result(Err(e))),
            },
        };

        let chain_id = match self.chain_id() {
            Ok(chain_id) => chain_id,
            Err(e) => return Box::new(future::result(Err(e))),
        };

        let message = ethereum::TransactionMessage {
            nonce,
            gas_price: request.gas_price.unwrap_or(U256::from(1)),
            gas_limit: request.gas.unwrap_or(U256::max_value()),
            value: request.value.unwrap_or(U256::zero()),
            input: request.data.map(|s| s.into_vec()).unwrap_or_default(),
            action: match request.to {
                Some(to) => ethereum::TransactionAction::Call(to),
                None => ethereum::TransactionAction::Create,
            },
            chain_id: chain_id.map(|s| s.as_u64()),
        };

        let mut transaction = None;
        for signer in &self.signers {
            if signer.address == from {
                match sign_transaction_message(message, &signer.private_key)
                    .map_err(|e| internal_err(e))
                {
                    Ok(tx) => transaction = Some(tx),
                    Err(e) => return Box::new(future::result(Err(e))),
                }
                break;
            }
        }

        let transaction = match transaction {
            Some(transaction) => transaction,
            None => {
                return Box::new(future::result(Err(internal_err(
                    "no signer available",
                ))));
            }
        };
        let transaction_hash =
            H256::from_slice(Keccak256::digest(&rlp::encode(&transaction)).as_slice());
        let function =
            baseapp::Action::Ethereum(module_ethereum::Action::Transact(transaction));
        let resp = match self
            .forwarder
            .forward_txn(UncheckedTransaction::new_unsigned(function), TX_SYNC)
            .map_err(|e| internal_err(e))
        {
            Ok(resp) => resp,
            Err(e) => return Box::new(future::result(Err(e))),
        };

        if resp.is_success() {
            Box::new(future::result(Ok(transaction_hash)))
        } else {
            Box::new(future::result(Err(internal_err(format!(
                "send ethereum transaction failed"
            )))))
        }
    }

    fn call(&self, _request: CallRequest, _: Option<BlockNumber>) -> Result<Bytes> {
        println!("invoked: fn call");
        Err(internal_err("Method not available."))
    }

    fn syncing(&self) -> Result<SyncStatus> {
        println!("invoked: fn syncing");
        Err(internal_err("Method not available."))
    }

    fn author(&self) -> Result<H160> {
        println!("invoked: fn author");
        Err(internal_err("Method not available."))
    }

    fn is_mining(&self) -> Result<bool> {
        println!("invoked: fn is_mining");
        Err(internal_err("Method not available."))
    }

    fn gas_price(&self) -> Result<U256> {
        println!("invoked: fn gas_price");
        Err(internal_err("Method not available."))
    }

    fn block_number(&self) -> Result<U256> {
        let height = self
            .account_base_app
            .read()
            .chain_state
            .read()
            .height()
            .map_err(|e| internal_err(e))?;
        Ok(U256::from(height))
    }

    fn storage_at(
        &self,
        _address: H160,
        _index: U256,
        _number: Option<BlockNumber>,
    ) -> Result<H256> {
        println!("invoked: fn storage_at");
        Err(internal_err("Method not available."))
    }

    fn block_by_hash(&self, _hash: H256, _full: bool) -> Result<Option<RichBlock>> {
        println!("invoked: fn block_by_hash");
        Err(internal_err("Method not available."))
    }

    fn block_by_number(
        &self,
        _number: BlockNumber,
        _full: bool,
    ) -> Result<Option<RichBlock>> {
        println!("invoked: fn block_by_number");
        Err(internal_err("Method not available."))
    }

    fn transaction_count(
        &self,
        address: H160,
        number: Option<BlockNumber>,
    ) -> Result<U256> {
        let account_id = EthereumAddressMapping::into_account_id(address);

        let ctx = if let Some(BlockNumber::Pending) = number {
            Some(self.account_base_app.read().check_state.clone())
        } else {
            None
        };
        let sa = self
            .account_base_app
            .read()
            .account_of(&account_id, ctx)
            .map_err(|e| internal_err(e))?;
        Ok(U256::from(sa.nonce))
    }

    fn block_transaction_count_by_hash(&self, _hash: H256) -> Result<Option<U256>> {
        println!("invoked: fn block_transaction_count_by_hash");
        Err(internal_err("Method not available."))
    }

    fn block_transaction_count_by_number(
        &self,
        _number: BlockNumber,
    ) -> Result<Option<U256>> {
        println!("invoked: fn block_transaction_count_by_number");
        Err(internal_err("Method not available."))
    }

    fn block_uncles_count_by_hash(&self, _: H256) -> Result<U256> {
        println!("invoked: fn block_uncles_count_by_hash");
        Err(internal_err("Method not available."))
    }

    fn block_uncles_count_by_number(&self, _: BlockNumber) -> Result<U256> {
        println!("invoked: fn block_uncles_count_by_number");
        Err(internal_err("Method not available."))
    }

    fn code_at(&self, _address: H160, _number: Option<BlockNumber>) -> Result<Bytes> {
        println!("invoked: fn code_at");
        Err(internal_err("Method not available."))
    }

    fn send_raw_transaction(&self, _bytes: Bytes) -> BoxFuture<H256> {
        println!("invoked: fn code_at");
        Box::new(future::result(Err(internal_err("Method not available."))))
    }

    fn estimate_gas(
        &self,
        _request: CallRequest,
        _: Option<BlockNumber>,
    ) -> Result<U256> {
        Ok(U256::from(10))

        // println!("invoked: fn estimate_gas");
        // Err(internal_err("Method not available."))
    }

    fn transaction_by_hash(&self, _hash: H256) -> Result<Option<Transaction>> {
        println!("invoked: fn transaction_by_hash");
        Err(internal_err("Method not available."))
    }

    fn transaction_by_block_hash_and_index(
        &self,
        _hash: H256,
        _index: Index,
    ) -> Result<Option<Transaction>> {
        println!("invoked: fn transaction_by_block_hash_and_index");
        Err(internal_err("Method not available."))
    }

    fn transaction_by_block_number_and_index(
        &self,
        _number: BlockNumber,
        _index: Index,
    ) -> Result<Option<Transaction>> {
        println!("invoked: fn transaction_by_block_number_and_index");
        Err(internal_err("Method not available."))
    }

    fn transaction_receipt(&self, _hash: H256) -> Result<Option<Receipt>> {
        println!("invoked: fn transaction_receipt");
        Err(internal_err("Method not available."))
    }

    fn uncle_by_block_hash_and_index(
        &self,
        _: H256,
        _: Index,
    ) -> Result<Option<RichBlock>> {
        Ok(None)
    }

    fn uncle_by_block_number_and_index(
        &self,
        _: BlockNumber,
        _: Index,
    ) -> Result<Option<RichBlock>> {
        Ok(None)
    }

    fn logs(&self, _filter: Filter) -> Result<Vec<Log>> {
        println!("invoked: fn logs");
        Err(internal_err("Method not available."))
    }

    fn work(&self) -> Result<Work> {
        Ok(Work {
            pow_hash: H256::default(),
            seed_hash: H256::default(),
            target: H256::default(),
            number: None,
        })
    }

    fn submit_work(&self, _: H64, _: H256, _: H256) -> Result<bool> {
        Ok(false)
    }

    fn submit_hashrate(&self, _: U256, _: H256) -> Result<bool> {
        Ok(false)
    }
}

pub struct NetApiImpl;

impl NetApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NetApiImpl {
    fn default() -> Self {
        NetApiImpl::new()
    }
}

impl NetApi for NetApiImpl {
    fn is_listening(&self) -> Result<bool> {
        println!("invoked: fn is_listening");
        Ok(true)
    }

    fn peer_count(&self) -> Result<PeerCount> {
        println!("invoked: fn peer_count");
        Ok(PeerCount::String(format!("0x{:x}", 1)))
    }

    fn version(&self) -> Result<String> {
        println!("invoked: fn version");
        Ok(String::from("1336"))
    }
}

pub struct Web3ApiImpl;

impl Web3ApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Web3ApiImpl {
    fn default() -> Self {
        Web3ApiImpl::new()
    }
}

impl Web3Api for Web3ApiImpl {
    fn client_version(&self) -> Result<String> {
        println!("invoked: fn client_version");
        Ok(String::from("findora-eth-api/v0.1.0-rust"))
    }

    fn sha3(&self, input: Bytes) -> Result<H256> {
        println!("invoked: fn sha3");
        Ok(H256::from_slice(
            Keccak256::digest(&input.into_vec()).as_slice(),
        ))
    }
}

pub struct EthFilterApiImpl;

impl EthFilterApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EthFilterApiImpl {
    fn default() -> Self {
        EthFilterApiImpl::new()
    }
}

impl EthFilterApi for EthFilterApiImpl {
    fn new_filter(&self, _filter: Filter) -> Result<U256> {
        println!("invoked: fn new_filter");
        Ok(U256::zero())
    }

    fn new_block_filter(&self) -> Result<U256> {
        println!("invoked: fn new_block_filter");
        Ok(U256::zero())
    }

    fn new_pending_transaction_filter(&self) -> Result<U256> {
        println!("invoked: fn new_pending_transaction_filter");
        Err(internal_err("Method not available."))
    }

    fn filter_changes(&self, _index: Index) -> Result<FilterChanges> {
        println!("invoked: fn filter_changes");
        Err(internal_err("Method not available."))
    }

    fn filter_logs(&self, _index: Index) -> Result<Vec<Log>> {
        println!("invoked: fn filter_logs");
        Err(internal_err("Method not available."))
    }

    fn uninstall_filter(&self, _index: Index) -> Result<bool> {
        println!("invoked: fn uninstall_filter");
        Err(internal_err("Method not available."))
    }
}
