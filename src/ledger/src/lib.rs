#![deny(warnings)]

#[macro_use]
pub mod data_model;

pub mod staking;

#[cfg(not(target_arch = "wasm32"))]
pub mod store;

pub mod address;
pub mod utils;
