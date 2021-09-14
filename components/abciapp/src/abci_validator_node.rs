#![deny(warnings)]
#![allow(clippy::field_reassign_with_default)]

use ruc::*;

pub mod abci;

fn main() {
    fp_utils::logging::init_logging(None);

    pnk!(abci::run());
}
