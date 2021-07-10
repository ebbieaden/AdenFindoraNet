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

use std::{marker::PhantomData, sync::Arc, iter};
use std::collections::BTreeMap;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rustc_hex::ToHex;
use sp_runtime::traits::{
	Block as BlockT, BlakeTwo256,
	UniqueSaturatedInto
};
use sp_transaction_pool::TransactionPool;
use sp_api::{ProvideRuntimeApi, BlockId};
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sc_client_api::{
	backend::{StorageProvider, Backend, StateBackend},
	client::BlockchainEvents
};
use sc_rpc::Metadata;
use log::warn;

use jsonrpc_pubsub::{
	typed::Subscriber, SubscriptionId,
	manager::{SubscriptionManager, IdProvider}
};
use fc_rpc_core::EthPubSubApi::{self as EthPubSubApiT};
use fc_rpc_core::types::{
	Rich, Header, Bytes, Log, FilteredParams,
	pubsub::{Kind, Params, Result as PubSubResult, PubSubSyncStatus}
};
use ethereum_types::{H256, U256};
use sha3::{Keccak256, Digest};

pub use fc_rpc_core::EthPubSubApiServer;
use futures::{StreamExt as _, TryStreamExt as _};

use jsonrpc_core::{Result as JsonRpcResult, futures::{Future, Sink}};
use fp_rpc::EthereumRuntimeRPCApi;

use sc_network::{NetworkService, ExHashT};


pub struct EthPubSubApiImpl;

impl EthPubSubApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EthPubSubApiImpl {
    fn default() -> Self {
        EthPubSubApiImpl::new()
    }
}

impl EthPubSubApiT for EthPubSubApiImpl {
	type Metadata = Metadata;
	fn subscribe(
		&self,
		_metadata: Self::Metadata,
		subscriber: Subscriber<PubSubResult>,
		kind: Kind,
		params: Option<Params>,
	) {
        println!("invoked: fn subscribe");
	}

	fn unsubscribe(
		&self,
		_metadata: Option<Self::Metadata>,
		subscription_id: SubscriptionId
	) -> JsonRpcResult<bool> {
        println!("invoked: fn unsubscribe");
	}
}
