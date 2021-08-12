<<<<<<<< HEAD:src/components/abciapp/src/bins/findorad.rs
use abciapp::abci;
========
#![deny(warnings)]
#![allow(clippy::field_reassign_with_default)]

use ruc::*;

pub mod abci;
>>>>>>>> 42b24bd8 (merge develop refactor code (#453)):components/abciapp/src/abci_validator_node.rs

fn main() {
    utils::logging::init_logging(None);

    pnk!(abci::run());
}
