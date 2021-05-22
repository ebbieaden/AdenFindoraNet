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
use ruc::*;
use std::{convert::TryInto, fs};

lazy_static! {
    static ref PATH: String = format!(
        "{}/.__tendermint_height__",
        crate::abci::LEDGER_DIR
            .as_deref()
            .map(|ld| {
                pnk!(fs::create_dir_all(ld));
                ld
            })
            .unwrap_or("/tmp")
    );
}

pub(super) fn write_height(h: i64) -> Result<()> {
    fs::write(&*PATH, i64::to_ne_bytes(h)).c(d!())
}

pub(super) fn read_height() -> Result<i64> {
    fs::read(&*PATH)
        .c(d!())
        .map(|b| i64::from_ne_bytes(b.try_into().unwrap()))
}
