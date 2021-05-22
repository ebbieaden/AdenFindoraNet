//!
//! # Tendermint Node Address
//!
//! - sha256(pubkey)[:20]
//!

use ledger::staking::td_addr_to_bytes;
use ruc::*;
use serde::Deserialize;
use std::env;

pub fn get_self_addr() -> Result<Vec<u8>> {
    from_env()
        .c(d!())
        .or_else(|_| from_tendermint_rpc().c(d!()))
}

fn from_env() -> Result<Vec<u8>> {
    const VAR: &str = "TD_NODE_SELF_ADDR";
    env::var(VAR)
        .c(d!())
        .and_then(|td_addr| td_addr_to_bytes(&td_addr).c(d!()))
}

fn from_tendermint_rpc() -> Result<Vec<u8>> {
    const URL: &str = "http://node:26657/status";
    http_req(URL)
        .c(d!())
        .and_then(|ni| td_addr_to_bytes(&ni.result.validator_info.address).c(d!()))
}

// `curl node:26657/status`
//
// ```
// {
//   "result": {
//     "validator_info": {
//       "address": "52C155268A12B210DE36FFB152A3CB913AFCFB17",
//       "pub_key": {
//         "type": "tendermint/PubKeyEd25519",
//         "value": "c7QbZH/7TNaS//LXyXWcL+ZiEiiZfXv3p57t2eAxB+8="
//       },
//       "voting_power": "0"
//     }
//   }
// }
// ```
fn http_req(url: &str) -> Result<NodeInfo> {
    attohttpc::get(url)
        .send()
        .c(d!())?
        .error_for_status()
        .c(d!())?
        .bytes()
        .c(d!())
        .and_then(|b| serde_json::from_slice(&b).c(d!()))
}

#[derive(Deserialize)]
struct NodeInfo {
    result: ValidatorInfo,
}

#[derive(Deserialize)]
struct ValidatorInfo {
    validator_info: ValidatorAddr,
}

#[derive(Deserialize)]
struct ValidatorAddr {
    address: String,
    // pub_key: ValidatorPubKey,
}

// #[derive(Deserialize)]
// struct ValidatorPubKey {
//     value: String,
// }
