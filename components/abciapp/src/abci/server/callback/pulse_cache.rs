//!
//! ## issue
//!
//! - abci process need a very long time to finish a restarting
//!
//! ## reason
//!
//! #### 1. incorrect pulse count
//!
//! - pulse count will not be stored to disk
//! until there are some real transactions
//! - this will cause to send a block-height smaller
//! than the real one to tendermint in `ABCI::info` callback
//! - and this will cause to replay many unnecessary blocks
//! - and this will take a long time ...
//!
//! #### 2. replay all real transactions at starting
//!
//! - TODO: implement a state snapshot to avoid replay
//!
//! ## fix
//!
//! - cache block-height to disk along with the `ABCI::commit` callback
//! - send this cached block height to tendermint when restarting
//!

use lazy_static::lazy_static;
use ledger::staking::Staking;
use ruc::*;
use std::{convert::TryInto, fs};

lazy_static! {
    static ref PATH: (String, String) = {
        let ld = crate::abci::LEDGER_DIR.as_deref().unwrap_or("/tmp");
        pnk!(fs::create_dir_all(ld));

        let height_cache = format!("{}/.__tendermint_height__", &ld);
        let staking_cache = format!("{}/.____staking____", &ld);

        (height_cache, staking_cache)
    };
}

pub(super) fn write_height(h: i64) -> Result<()> {
    fs::write(&PATH.0, i64::to_ne_bytes(h)).c(d!())
}

pub(super) fn read_height() -> Result<i64> {
    fs::read(&PATH.0)
        .c(d!())
        .map(|b| i64::from_ne_bytes(b.try_into().unwrap()))
}

pub(super) fn write_staking(s: &Staking) -> Result<()> {
    serde_json::to_vec(s)
        .c(d!())
        .and_then(|bytes| fs::write(&PATH.1, bytes).c(d!()))
}

pub(super) fn read_staking() -> Result<Staking> {
    fs::read(&PATH.1)
        .c(d!())
        .and_then(|bytes| serde_json::from_slice(&bytes).c(d!()))
}
