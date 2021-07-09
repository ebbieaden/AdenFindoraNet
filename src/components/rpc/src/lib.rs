// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//use std::time;
use ethereum_types::{H160, H256, U256, U64};
use fc_rpc_core::types::PeerCount;
use fc_rpc_core::types::{BlockNumber, Bytes, CallRequest, TransactionRequest};
use fc_rpc_core::{EthApi as EthApiT, NetApi as NetApiT, Web3Api as Web3ApiT};
use jsonrpc_core::{futures::future, BoxFuture, Error, ErrorCode, Result};
use rustc_hex::FromHex;
use sha3::{Digest, Keccak256};
use std::str::FromStr;

pub fn internal_err<T: ToString>(message: T) -> Error {
    Error {
        code: ErrorCode::InternalError,
        message: message.to_string(),
        data: None,
    }
}

pub struct EthApiImpl;

impl EthApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl EthApiT for EthApiImpl {
    fn protocol_version(&self) -> Result<u64> {
        println!("invoked: fn protocol_version");
        Ok(1)
    }

    fn hashrate(&self) -> Result<U256> {
        println!("invoked: fn hashrate");
        Ok(U256::zero())
    }

    fn chain_id(&self) -> Result<Option<U64>> {
        // let hash = self.client.info().best_hash;
        // Ok(Some(self.client.runtime_api().chain_id(&BlockId::Hash(hash))
        //         .map_err(|err| internal_err(format!("fetch runtime chain id failed: {:?}", err)))?.into()))
        println!("invoked: fn chain_id");
        Ok(Some(0x538.into()))
    }

    fn accounts(&self) -> Result<Vec<H160>> {
        println!("invoked: fn accounts");
        let acc1 = H160::from_str("671Fb64365c7656C0D955aDcBcae8e3F62fF6A1B").unwrap();
        let acc2 = H160::from_str("05C7dBdd1954D59c9afaB848dA7d8DD3F35e69Cd").unwrap();
        Ok(vec![acc1, acc2])
        // let mut accounts = Vec::new();
        // for signer in &self.signers {
        //     accounts.append(&mut signer.accounts());
        // }
        // Ok(accounts)
        //println!("invoked: fn accounts");
        //Ok(Vec::new())
    }

    fn balance(&self, address: H160, number: Option<BlockNumber>) -> Result<U256> {
        // if let Ok(Some(id)) = frontier_backend_client::native_block_id::<B, C>(self.client.as_ref(), self.backend.as_ref(), number) {
        //     return Ok(
        //         self.client
        //             .runtime_api()
        //             .account_basic(&id, address)
        //             .map_err(|err| internal_err(format!("fetch runtime chain id failed: {:?}", err)))?
        //             .balance.into()
        //     )
        // }
        println!("invoked: fn balance: {}", address.to_string());
        Ok(U256::from(2340000000000000_u128))
    }

    fn send_transaction(&self, request: TransactionRequest) -> BoxFuture<H256> {
        // let from = match request.from {
        //     Some(from) => from,
        //     None => {
        //         let accounts = match self.accounts() {
        //             Ok(accounts) => accounts,
        //             Err(e) => return Box::new(future::result(Err(e))),
        //         };

        //         match accounts.get(0) {
        //             Some(account) => account.clone(),
        //             None => return Box::new(future::result(Err(internal_err("no signer available")))),
        //         }
        //     },
        // };

        // let nonce = match request.nonce {
        //     Some(nonce) => nonce,
        //     None => {
        //         match self.transaction_count(from, None) {
        //             Ok(nonce) => nonce,
        //             Err(e) => return Box::new(future::result(Err(e))),
        //         }
        //     },
        // };

        // let chain_id = match self.chain_id() {
        //     Ok(chain_id) => chain_id,
        //     Err(e) => return Box::new(future::result(Err(e))),
        // };

        // let message = ethereum::TransactionMessage {
        //     nonce,
        //     gas_price: request.gas_price.unwrap_or(U256::from(1)),
        //     gas_limit: request.gas.unwrap_or(U256::max_value()),
        //     value: request.value.unwrap_or(U256::zero()),
        //     input: request.data.map(|s| s.into_vec()).unwrap_or_default(),
        //     action: match request.to {
        //         Some(to) => ethereum::TransactionAction::Call(to),
        //         None => ethereum::TransactionAction::Create,
        //     },
        //     chain_id: chain_id.map(|s| s.as_u64()),
        // };

        // let mut transaction = None;

        // for signer in &self.signers {
        //     if signer.accounts().contains(&from) {
        //         match signer.sign(message, &from) {
        //             Ok(t) => transaction = Some(t),
        //             Err(e) => return Box::new(future::result(Err(e))),
        //         }
        //         break
        //     }
        // }

        // let transaction = match transaction {
        //     Some(transaction) => transaction,
        //     None => return Box::new(future::result(Err(internal_err("no signer available")))),
        // };
        // let transaction_hash = H256::from_slice(
        //     Keccak256::digest(&rlp::encode(&transaction)).as_slice()
        // );
        // let hash = self.client.info().best_hash;
        // let number = self.client.info().best_number;
        // let pending = self.pending_transactions.clone();
        // Box::new(
        //     self.pool
        //         .submit_one(
        //             &BlockId::hash(hash),
        //             TransactionSource::Local,
        //             self.convert_transaction.convert_transaction(transaction.clone()),
        //         )
        //         .compat()
        //         .map(move |_| {
        //             if let Some(pending) = pending {
        //                 if let Ok(locked) = &mut pending.lock() {
        //                     locked.insert(
        //                         transaction_hash,
        //                         PendingTransaction::new(
        //                             transaction_build(transaction, None, None),
        //                             UniqueSaturatedInto::<u64>::unique_saturated_into(
        //                                 number
        //                             )
        //                         )
        //                     );
        //                 }
        //             }
        //             transaction_hash
        //         })
        //         .map_err(|err| internal_err(format!("submit transaction to pool failed: {:?}", err)))
        // )

        println!("invoked: fn send_transaction");
        Box::new(future::result(Err(internal_err("unimplemented"))))
    }

    fn call(&self, request: CallRequest, _: Option<BlockNumber>) -> Result<Bytes> {
        // let hash = self.client.info().best_hash;

        // let CallRequest {
        //     from,
        //     to,
        //     gas_price,
        //     gas,
        //     value,
        //     data,
        //     nonce
        // } = request;

        // // use given gas limit or query current block's limit
        // let gas_limit = match gas {
        //     Some(amount) => amount,
        //     None => {
        //         let block = self.client.runtime_api().current_block(&BlockId::Hash(hash))
        //             .map_err(|err| internal_err(format!("runtime error: {:?}", err)))?;
        //         if let Some(block) = block {
        //             block.header.gas_limit
        //         } else {
        //             return Err(internal_err(format!("block unavailable, cannot query gas limit")));
        //         }
        //     },
        // };
        // let data = data.map(|d| d.0).unwrap_or_default();

        // match to {
        //     Some(to) => {
        //         let info = self.client.runtime_api()
        //             .call(
        //                 &BlockId::Hash(hash),
        //                 from.unwrap_or_default(),
        //                 to,
        //                 data,
        //                 value.unwrap_or_default(),
        //                 gas_limit,
        //                 gas_price,
        //                 nonce,
        //                 false,
        //             )
        //             .map_err(|err| internal_err(format!("runtime error: {:?}", err)))?
        //             .map_err(|err| internal_err(format!("execution fatal: {:?}", err)))?;

        //         error_on_execution_failure(&info.exit_reason, &info.value)?;

        //         Ok(Bytes(info.value))
        //     },
        //     None => {
        //         let info = self.client.runtime_api()
        //             .create(
        //                 &BlockId::Hash(hash),
        //                 from.unwrap_or_default(),
        //                 data,
        //                 value.unwrap_or_default(),
        //                 gas_limit,
        //                 gas_price,
        //                 nonce,
        //                 false,
        //             )
        //             .map_err(|err| internal_err(format!("runtime error: {:?}", err)))?
        //             .map_err(|err| internal_err(format!("execution fatal: {:?}", err)))?;

        //         error_on_execution_failure(&info.exit_reason, &[])?;

        //         Ok(Bytes(info.value[..].to_vec()))
        //     },
        // }

        println!("invoked: fn call");
        Err(internal_err("unimplemented".to_string()))
    }
}

pub struct NetApiImpl;

impl NetApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl NetApiT for NetApiImpl {
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

impl Web3ApiT for Web3ApiImpl {
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
