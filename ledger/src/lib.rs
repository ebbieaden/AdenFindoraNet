#![deny(warnings)]

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[macro_use]
pub mod data_model;
pub mod policies;
pub mod policy_script;

pub mod staking;
pub mod store;

pub mod address;
pub mod utils;
