use crate::{error_on_execution_failure, forward::*, internal_err};
use baseapp::{BaseApp, BaseProvider, UncheckedTransaction};
use ethereum::{
    Block as EthereumBlock, Transaction as EthereumTransaction,
    TransactionMessage as EthereumTransactionMessage,
};
use ethereum_types::{H160, H256, H512, H64, U256, U64};
use fp_evm::TransactionStatus;
use fp_rpc_core::types::{
    Block, BlockNumber, BlockTransactions, Bytes, CallRequest, Filter, FilterChanges,
    FilteredParams, Index, Log, PeerCount, Receipt, Rich, RichBlock, SyncStatus,
    Transaction, TransactionRequest, Work,
};
use fp_rpc_core::{EthApi, EthFilterApi, NetApi, Web3Api};
use fp_traits::evm::{
    AddressMapping, DecimalsMapping, EthereumAddressMapping, FeeCalculator,
};
use fp_utils::ethereum::{sign_transaction_message, KeyPair};
use jsonrpc_core::{futures::future, BoxFuture, Result};
use log::debug;
use module_evm::{Call, Create, Runner};
use parking_lot::RwLock;
use sha3::{Digest, Keccak256};
use std::collections::BTreeMap;
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
        Ok(self.account_base_app.read().app_version)
    }

    fn hashrate(&self) -> Result<U256> {
        Ok(U256::zero())
    }

    fn chain_id(&self) -> Result<Option<U64>> {
        Ok(Some(<BaseApp as module_evm::Config>::ChainId::get().into()))
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
            Some(self.account_base_app.read().deliver_state.clone())
        } else {
            None
        };

        let account_id = EthereumAddressMapping::into_account_id(address);
        if let Ok(sa) = self.account_base_app.read().account_of(&account_id, ctx) {
            Ok(
                <BaseApp as module_evm::Config>::DecimalsMapping::from_native_token(
                    U256::from(sa.balance),
                )
                .unwrap_or_default(),
            )
        } else {
            Ok(U256::zero())
        }
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
            gas_price: request.gas_price.unwrap_or(
                <BaseApp as module_evm::Config>::FeeCalculator::min_gas_price(),
            ),
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

    fn call(&self, request: CallRequest, _: Option<BlockNumber>) -> Result<Bytes> {
        let CallRequest {
            from,
            to,
            gas_price,
            gas,
            value,
            data,
            nonce,
        } = request;

        let value = <BaseApp as module_evm::Config>::DecimalsMapping::into_native_token(
            value.unwrap_or_default(),
        );

        // use given gas limit or query current block's limit
        let gas_limit = match gas {
            Some(amount) => amount,
            None => {
                let block = self.account_base_app.read().current_block();
                if let Some(block) = block {
                    block.header.gas_limit
                } else {
                    <BaseApp as module_evm::Config>::BlockGasLimit::get()
                }
            }
        };
        let data = data.map(|d| d.0).unwrap_or_default();

        let mut config = <BaseApp as module_evm::Config>::config().clone();
        config.estimate = true;

        let ctx = self
            .account_base_app
            .read()
            .create_query_context(0, false)
            .unwrap();
        match to {
            Some(to) => {
                let call = Call {
                    source: from.unwrap_or_default(),
                    target: to,
                    input: data,
                    value,
                    gas_limit: gas_limit.as_u64(),
                    gas_price,
                    nonce,
                };

                let info =
                    <BaseApp as module_evm::Config>::Runner::call(&ctx, call, &config)
                        .map_err(|err| {
                        internal_err(format!("evm runner call error: {:?}", err))
                    })?;
                debug!(target: "eth_rpc", "evm runner call result: {:?}", info);

                error_on_execution_failure(&info.exit_reason, &info.value)?;

                Ok(Bytes(info.value))
            }
            None => {
                let create = Create {
                    source: from.unwrap_or_default(),
                    init: data,
                    value,
                    gas_limit: gas_limit.as_u64(),
                    gas_price,
                    nonce,
                };

                let info = <BaseApp as module_evm::Config>::Runner::create(
                    &ctx, create, &config,
                )
                .map_err(|err| {
                    internal_err(format!("evm runner create error: {:?}", err))
                })?;
                debug!(target: "eth_rpc", "evm runner create result: {:?}", info);

                error_on_execution_failure(&info.exit_reason, &[])?;

                Ok(Bytes(info.value[..].to_vec()))
            }
        }
    }

    fn syncing(&self) -> Result<SyncStatus> {
        // TODO
        Ok(SyncStatus::None)
    }

    fn author(&self) -> Result<H160> {
        let proposer = self
            .account_base_app
            .read()
            .check_state
            .header
            .proposer_address
            .clone();
        Ok(H160::from_slice(&proposer[0..20]))
    }

    fn is_mining(&self) -> Result<bool> {
        // TODO
        Ok(false)
    }

    fn gas_price(&self) -> Result<U256> {
        Ok(<BaseApp as module_evm::Config>::FeeCalculator::min_gas_price())
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
        // TODO
        Ok(H256::default())
    }

    fn block_by_hash(&self, hash: H256, full: bool) -> Result<Option<RichBlock>> {
        let block = self.account_base_app.read().current_block();
        let statuses = self.account_base_app.read().current_transaction_statuses();

        match (block, statuses) {
            (Some(block), Some(statuses)) => Ok(Some(rich_block_build(
                block,
                statuses.into_iter().map(|s| Some(s)).collect(),
                Some(hash),
                full,
            ))),
            _ => Ok(None),
        }
    }

    fn block_by_number(
        &self,
        number: BlockNumber,
        full: bool,
    ) -> Result<Option<RichBlock>> {
        let block = self.account_base_app.read().current_block();
        let statuses = self.account_base_app.read().current_transaction_statuses();

        match (block, statuses) {
            (Some(block), Some(statuses)) => {
                let hash = block.header.hash();

                match number {
                    BlockNumber::Hash { hash: h, .. } => {
                        if h != hash {
                            return Ok(None);
                        }
                    }
                    BlockNumber::Num(num) => {
                        if block.header.number != U256::from(num) {
                            return Ok(None);
                        }
                    }
                    BlockNumber::Latest => {}
                    _ => return Ok(None),
                }

                Ok(Some(rich_block_build(
                    block,
                    statuses.into_iter().map(|s| Some(s)).collect(),
                    Some(hash),
                    full,
                )))
            }
            _ => Ok(None),
        }
    }

    fn transaction_count(
        &self,
        address: H160,
        number: Option<BlockNumber>,
    ) -> Result<U256> {
        let account_id =
            <BaseApp as module_evm::Config>::AddressMapping::into_account_id(address);

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

    fn block_transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>> {
        let block = self.account_base_app.read().current_block();
        match block {
            Some(block) => {
                if hash != block.header.hash() {
                    return Ok(None);
                }
                Ok(Some(U256::from(block.transactions.len())))
            }
            None => Ok(None),
        }
    }

    fn block_transaction_count_by_number(
        &self,
        number: BlockNumber,
    ) -> Result<Option<U256>> {
        let block = self.account_base_app.read().current_block();
        match block {
            Some(block) => {
                let hash = block.header.hash();

                match number {
                    BlockNumber::Hash { hash: h, .. } => {
                        if h != hash {
                            return Ok(None);
                        }
                    }
                    BlockNumber::Num(num) => {
                        if block.header.number != U256::from(num) {
                            return Ok(None);
                        }
                    }
                    BlockNumber::Latest => {}
                    _ => {
                        return Ok(None);
                    }
                }

                Ok(Some(U256::from(block.transactions.len())))
            }
            None => Ok(None),
        }
    }

    fn block_uncles_count_by_hash(&self, _: H256) -> Result<U256> {
        Ok(U256::zero())
    }

    fn block_uncles_count_by_number(&self, _: BlockNumber) -> Result<U256> {
        Ok(U256::zero())
    }

    fn code_at(&self, address: H160, _number: Option<BlockNumber>) -> Result<Bytes> {
        Ok(self
            .account_base_app
            .read()
            .account_code_at(address)
            .unwrap_or(vec![])
            .into())
    }

    fn send_raw_transaction(&self, bytes: Bytes) -> BoxFuture<H256> {
        let transaction = match rlp::decode::<ethereum::Transaction>(&bytes.0[..]) {
            Ok(transaction) => transaction,
            Err(_) => {
                return Box::new(future::result(Err(internal_err(
                    "decode transaction failed",
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
                "send ethereum raw transaction failed"
            )))))
        }
    }

    fn estimate_gas(
        &self,
        request: CallRequest,
        _: Option<BlockNumber>,
    ) -> Result<U256> {
        let CallRequest {
            from,
            to,
            gas_price: _,
            gas: _,
            value,
            data,
            nonce: _,
        } = request;

        let value = <BaseApp as module_evm::Config>::DecimalsMapping::into_native_token(
            value.unwrap_or_default(),
        );

        let gas_limit = <BaseApp as module_evm::Config>::BlockGasLimit::get();

        let data = data.map(|d| d.0).unwrap_or_default();

        let mut config = <BaseApp as module_evm::Config>::config().clone();
        config.estimate = true;

        let ctx = self
            .account_base_app
            .read()
            .create_query_context(0, false)
            .unwrap();

        let used_gas = match to {
            Some(to) => {
                let call = Call {
                    source: from.unwrap_or_default(),
                    target: to,
                    input: data,
                    value,
                    gas_limit: gas_limit.as_u64(),
                    gas_price: None,
                    nonce: None,
                };

                let info =
                    <BaseApp as module_evm::Config>::Runner::call(&ctx, call, &config)
                        .map_err(|err| {
                        internal_err(format!("evm runner call error: {:?}", err))
                    })?;
                debug!(target: "eth_rpc", "evm runner call result: {:?}", info);

                error_on_execution_failure(&info.exit_reason, &info.value)?;

                info.used_gas
            }
            None => {
                let create = Create {
                    source: from.unwrap_or_default(),
                    init: data,
                    value,
                    gas_limit: gas_limit.as_u64(),
                    gas_price: None,
                    nonce: None,
                };

                let info = <BaseApp as module_evm::Config>::Runner::create(
                    &ctx, create, &config,
                )
                .map_err(|err| {
                    internal_err(format!("evm runner create error: {:?}", err))
                })?;
                debug!(target: "eth_rpc", "evm runner create result: {:?}", info);

                error_on_execution_failure(&info.exit_reason, &[])?;

                info.used_gas
            }
        };

        Ok(used_gas)
    }

    fn transaction_by_hash(&self, hash: H256) -> Result<Option<Transaction>> {
        let block = self.account_base_app.read().current_block();
        let statuses = self.account_base_app.read().current_transaction_statuses();

        match (block, statuses) {
            (Some(block), Some(statuses)) => {
                let index = statuses.iter().position(|t| t.transaction_hash == hash);
                if index.is_none() {
                    return Ok(None);
                }

                Ok(Some(transaction_build(
                    block.transactions[index.unwrap()].clone(),
                    Some(block),
                    Some(statuses[index.unwrap()].clone()),
                )))
            }
            _ => Ok(None),
        }
    }

    fn transaction_by_block_hash_and_index(
        &self,
        hash: H256,
        index: Index,
    ) -> Result<Option<Transaction>> {
        let index = index.value();

        let block = self.account_base_app.read().current_block();
        let statuses = self.account_base_app.read().current_transaction_statuses();
        match (block, statuses) {
            (Some(block), Some(statuses)) => {
                if block.header.hash() != hash || index >= block.transactions.len() {
                    return Ok(None);
                }

                Ok(Some(transaction_build(
                    block.transactions[index].clone(),
                    Some(block),
                    Some(statuses[index].clone()),
                )))
            }
            _ => Ok(None),
        }
    }

    fn transaction_by_block_number_and_index(
        &self,
        number: BlockNumber,
        index: Index,
    ) -> Result<Option<Transaction>> {
        let index = index.value();

        let block = self.account_base_app.read().current_block();
        let statuses = self.account_base_app.read().current_transaction_statuses();
        match (block, statuses) {
            (Some(block), Some(statuses)) => {
                let hash = block.header.hash();
                match number {
                    BlockNumber::Hash { hash: h, .. } => {
                        if h != hash {
                            return Ok(None);
                        }
                    }
                    BlockNumber::Num(num) => {
                        if block.header.number != U256::from(num) {
                            return Ok(None);
                        }
                    }
                    BlockNumber::Latest => {}
                    _ => {
                        return Ok(None);
                    }
                }
                if index >= block.transactions.len() {
                    return Ok(None);
                }

                Ok(Some(transaction_build(
                    block.transactions[index].clone(),
                    Some(block),
                    Some(statuses[index].clone()),
                )))
            }
            _ => Ok(None),
        }
    }

    fn transaction_receipt(&self, hash: H256) -> Result<Option<Receipt>> {
        let block = self.account_base_app.read().current_block();
        let statuses = self.account_base_app.read().current_transaction_statuses();
        let receipts = self.account_base_app.read().current_receipts();

        match (block, statuses, receipts) {
            (Some(block), Some(statuses), Some(receipts)) => {
                let index = statuses.iter().position(|t| t.transaction_hash == hash);
                if index.is_none() {
                    return Ok(None);
                }

                let block_hash = H256::from_slice(
                    Keccak256::digest(&rlp::encode(&block.header)).as_slice(),
                );
                let receipt = receipts[index.unwrap()].clone();
                let status = statuses[index.unwrap()].clone();
                let mut cumulative_receipts = receipts.clone();
                cumulative_receipts.truncate((status.transaction_index + 1) as usize);

                return Ok(Some(Receipt {
                    transaction_hash: Some(status.transaction_hash),
                    transaction_index: Some(status.transaction_index.into()),
                    block_hash: Some(block_hash),
                    from: Some(status.from),
                    to: status.to,
                    block_number: Some(block.header.number),
                    cumulative_gas_used: {
                        let cumulative_gas: u32 = cumulative_receipts
                            .iter()
                            .map(|r| r.used_gas.as_u32())
                            .sum();
                        U256::from(cumulative_gas)
                    },
                    gas_used: Some(receipt.used_gas),
                    contract_address: status.contract_address,
                    logs: {
                        let mut pre_receipts_log_index = None;
                        if cumulative_receipts.len() > 0 {
                            cumulative_receipts.truncate(cumulative_receipts.len() - 1);
                            pre_receipts_log_index = Some(
                                cumulative_receipts
                                    .iter()
                                    .map(|r| r.logs.len() as u32)
                                    .sum::<u32>(),
                            );
                        }
                        receipt
                            .logs
                            .iter()
                            .enumerate()
                            .map(|(i, log)| Log {
                                address: log.address,
                                topics: log.topics.clone(),
                                data: Bytes(log.data.clone()),
                                block_hash: Some(block_hash),
                                block_number: Some(block.header.number),
                                transaction_hash: Some(status.transaction_hash),
                                transaction_index: Some(status.transaction_index.into()),
                                log_index: Some(U256::from(
                                    (pre_receipts_log_index.unwrap_or(0)) + i as u32,
                                )),
                                transaction_log_index: Some(U256::from(i)),
                                removed: false,
                            })
                            .collect()
                    },
                    status_code: Some(U64::from(receipt.state_root.to_low_u64_be())),
                    logs_bloom: receipt.logs_bloom,
                    state_root: None,
                }));
            }
            _ => Ok(None),
        }
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

    fn logs(&self, filter: Filter) -> Result<Vec<Log>> {
        let mut ret: Vec<Log> = Vec::new();
        if let Some(hash) = filter.block_hash.clone() {
            let block = self.account_base_app.read().current_block();
            let statuses = self.account_base_app.read().current_transaction_statuses();

            if let (Some(block), Some(statuses)) = (block, statuses) {
                if hash != block.header.hash() {
                    return Ok(ret);
                }

                filter_block_logs(&mut ret, &filter, block, statuses);
            }
        }
        Ok(ret)
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
        let chain_id = <BaseApp as module_evm::Config>::ChainId::get();
        Ok(String::from(chain_id.to_string()))
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
        Ok(String::from("findora-eth-api/v0.1.0-rust"))
    }

    fn sha3(&self, input: Bytes) -> Result<H256> {
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
        Err(internal_err("Method not available15."))
    }

    fn filter_changes(&self, _index: Index) -> Result<FilterChanges> {
        println!("invoked: fn filter_changes");
        Err(internal_err("Method not available16."))
    }

    fn filter_logs(&self, _index: Index) -> Result<Vec<Log>> {
        println!("invoked: fn filter_logs");
        Err(internal_err("Method not available17."))
    }

    fn uninstall_filter(&self, _index: Index) -> Result<bool> {
        println!("invoked: fn uninstall_filter");
        Err(internal_err("Method not available18."))
    }
}

fn rich_block_build(
    block: ethereum::Block,
    statuses: Vec<Option<TransactionStatus>>,
    hash: Option<H256>,
    full_transactions: bool,
) -> RichBlock {
    Rich {
        inner: Block {
            hash: Some(hash.unwrap_or_else(|| {
                H256::from_slice(
                    Keccak256::digest(&rlp::encode(&block.header)).as_slice(),
                )
            })),
            parent_hash: block.header.parent_hash,
            uncles_hash: block.header.ommers_hash,
            author: block.header.beneficiary,
            miner: block.header.beneficiary,
            state_root: block.header.state_root,
            transactions_root: block.header.transactions_root,
            receipts_root: block.header.receipts_root,
            number: Some(block.header.number),
            gas_used: block.header.gas_used,
            gas_limit: block.header.gas_limit,
            extra_data: Bytes(block.header.extra_data.clone()),
            logs_bloom: Some(block.header.logs_bloom),
            timestamp: U256::from(block.header.timestamp / 1000),
            difficulty: block.header.difficulty,
            total_difficulty: U256::zero(),
            seal_fields: vec![
                Bytes(block.header.mix_hash.as_bytes().to_vec()),
                Bytes(block.header.nonce.as_bytes().to_vec()),
            ],
            uncles: vec![],
            transactions: {
                if full_transactions {
                    BlockTransactions::Full(
                        block
                            .transactions
                            .iter()
                            .enumerate()
                            .map(|(index, transaction)| {
                                transaction_build(
                                    transaction.clone(),
                                    Some(block.clone()),
                                    Some(statuses[index].clone().unwrap_or_default()),
                                )
                            })
                            .collect(),
                    )
                } else {
                    BlockTransactions::Hashes(
                        block
                            .transactions
                            .iter()
                            .map(|transaction| {
                                H256::from_slice(
                                    Keccak256::digest(&rlp::encode(
                                        &transaction.clone(),
                                    ))
                                    .as_slice(),
                                )
                            })
                            .collect(),
                    )
                }
            },
            size: Some(U256::from(rlp::encode(&block).len() as u32)),
        },
        extra_info: BTreeMap::new(),
    }
}

fn transaction_build(
    transaction: EthereumTransaction,
    block: Option<EthereumBlock>,
    status: Option<TransactionStatus>,
) -> Transaction {
    let pubkey = match public_key(&transaction) {
        Ok(p) => Some(p),
        Err(_e) => None,
    };

    Transaction {
        hash: H256::from_slice(Keccak256::digest(&rlp::encode(&transaction)).as_slice()),
        nonce: transaction.nonce,
        block_hash: block.as_ref().map_or(None, |block| {
            Some(H256::from_slice(
                Keccak256::digest(&rlp::encode(&block.header)).as_slice(),
            ))
        }),
        block_number: block.as_ref().map(|block| block.header.number),
        transaction_index: status
            .as_ref()
            .map(|status| U256::from(status.transaction_index)),
        from: status.as_ref().map_or(
            {
                match pubkey {
                    Some(pk) => {
                        H160::from(H256::from_slice(Keccak256::digest(&pk).as_slice()))
                    }
                    _ => H160::default(),
                }
            },
            |status| status.from,
        ),
        to: status.as_ref().map_or(
            {
                match transaction.action {
                    ethereum::TransactionAction::Call(to) => Some(to),
                    _ => None,
                }
            },
            |status| status.to,
        ),
        value: transaction.value,
        gas_price: transaction.gas_price,
        gas: transaction.gas_limit,
        input: Bytes(transaction.clone().input),
        creates: status
            .as_ref()
            .map_or(None, |status| status.contract_address),
        raw: Bytes(rlp::encode(&transaction).to_vec()),
        public_key: pubkey.as_ref().map(|pk| H512::from(pk)),
        chain_id: transaction.signature.chain_id().map(U64::from),
        standard_v: U256::from(transaction.signature.standard_v()),
        v: U256::from(transaction.signature.v()),
        r: U256::from(transaction.signature.r().as_bytes()),
        s: U256::from(transaction.signature.s().as_bytes()),
    }
}

pub fn public_key(transaction: &EthereumTransaction) -> ruc::Result<[u8; 64]> {
    let mut sig = [0u8; 65];
    let mut msg = [0u8; 32];
    sig[0..32].copy_from_slice(&transaction.signature.r()[..]);
    sig[32..64].copy_from_slice(&transaction.signature.s()[..]);
    sig[64] = transaction.signature.standard_v();
    msg.copy_from_slice(
        &EthereumTransactionMessage::from(transaction.clone()).hash()[..],
    );

    fp_core::crypto::secp256k1_ecdsa_recover(&sig, &msg)
}

fn filter_block_logs<'a>(
    ret: &'a mut Vec<Log>,
    filter: &'a Filter,
    block: EthereumBlock,
    transaction_statuses: Vec<TransactionStatus>,
) -> &'a Vec<Log> {
    let params = FilteredParams::new(Some(filter.clone()));
    let mut block_log_index: u32 = 0;
    let block_hash =
        H256::from_slice(Keccak256::digest(&rlp::encode(&block.header)).as_slice());
    for status in transaction_statuses.iter() {
        let logs = status.logs.clone();
        let mut transaction_log_index: u32 = 0;
        let transaction_hash = status.transaction_hash;
        for ethereum_log in logs {
            let mut log = Log {
                address: ethereum_log.address.clone(),
                topics: ethereum_log.topics.clone(),
                data: Bytes(ethereum_log.data.clone()),
                block_hash: None,
                block_number: None,
                transaction_hash: None,
                transaction_index: None,
                log_index: None,
                transaction_log_index: None,
                removed: false,
            };
            let mut add: bool = true;
            if let (Some(_), Some(_)) = (filter.address.clone(), filter.topics.clone()) {
                if !params.filter_address(&log) || !params.filter_topics(&log) {
                    add = false;
                }
            } else if let Some(_) = filter.address {
                if !params.filter_address(&log) {
                    add = false;
                }
            } else if let Some(_) = &filter.topics {
                if !params.filter_topics(&log) {
                    add = false;
                }
            }
            if add {
                log.block_hash = Some(block_hash);
                log.block_number = Some(block.header.number.clone());
                log.transaction_hash = Some(transaction_hash);
                log.transaction_index = Some(U256::from(status.transaction_index));
                log.log_index = Some(U256::from(block_log_index));
                log.transaction_log_index = Some(U256::from(transaction_log_index));
                ret.push(log);
            }
            transaction_log_index += 1;
            block_log_index += 1;
        }
    }
    ret
}
