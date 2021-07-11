//!
//! # Findora Network Staking
//!
//! FNS, a command line tool for staking in findora network.
//!
//! ## Usage
//!
//! ```
//! fns [SUBCOMMAND]
//!
//! - stake
//!     - "--amount=[Amout]"
//!     - "--validator-pubkey=[Tendermint PubKey]"
//!     - "--validator-memo=[StakingMemo, default to empty]"
//! - claim
//!     - "--amount=[Amout <Optional, default to 'all'>]"
//! - unstake
//! - show, query real-time state of your staking
//! - setup
//!     - "--serv-addr=[URL/IP]"
//!     - "--owner-mnemonic-path=[File Path]"
//!         - the `id` of your validator will be drived from this
//! ```
//!

#![deny(warnings)]

use clap::{crate_authors, App, ArgGroup, SubCommand};
use fintools::fns;
use ruc::*;
use std::fmt;

fn main() {
    if let Err(e) = run() {
        tip_fail(e);
    } else {
        tip_success();
    }
}

fn run() -> Result<()> {
    let subcmd_genkey =
        SubCommand::with_name("genkey").about("Generate a random Xfr Key");
    let subcmd_stake_arggrp = ArgGroup::with_name("staking_flags")
        .args(&["commission-rate", "validator-memo"])
        .multiple(true)
        .conflicts_with("append");
    let subcmd_stake = SubCommand::with_name("stake")
        .arg_from_usage("-n, --amount=<Amount> 'how much `FRA unit`s you want to stake'")
        .arg_from_usage("-R, --commission-rate=[Rate] 'the commission rate of your node, a float number from 0.0 to 1.0'")
        .arg_from_usage("-M, --validator-memo=[Memo] 'the description of your node, optional'")
        .arg_from_usage("-a, --append 'stake more FRAs to your node'")
        .group(subcmd_stake_arggrp);
    let subcmd_unstake = SubCommand::with_name("unstake")
        .arg_from_usage("-n, --amount=[Amount] 'how much FRA to unstake, needed for partial undelegation'");
    let subcmd_claim = SubCommand::with_name("claim")
        .arg_from_usage("-n, --amount=[Amount] 'how much `FRA unit`s to claim'");
    let subcmd_show = SubCommand::with_name("show");
    let subcmd_setup = SubCommand::with_name("setup")
        .arg_from_usage(
            "-S, --serv-addr=[URL/IP] 'a fullnode address of Findora Network'",
        )
        .arg_from_usage(
            "-O, --owner-mnemonic-path=[Path], 'storage path of your mnemonic words'",
        )
        .arg_from_usage(
            "-K, --validator-key=[Path], 'path to the tendermint keys of your validator node'",
        );
    let subcmd_transfer = SubCommand::with_name("transfer")
        .arg_from_usage("-t, --target-addr=<Addr> 'wallet address of the receiver'")
        .arg_from_usage("-n, --amount=<Amount> 'how much FRA units to transfer'");
    let subcmd_set_initial_validators = SubCommand::with_name("set-initial-validators");

    let matches = App::new("fns")
        .version(fns::version())
        .author(crate_authors!())
        .about("A command line tool for staking in findora network.")
        .subcommand(subcmd_genkey)
        .subcommand(subcmd_stake)
        .subcommand(subcmd_unstake)
        .subcommand(subcmd_claim)
        .subcommand(subcmd_show)
        .subcommand(subcmd_setup)
        .subcommand(subcmd_transfer)
        .subcommand(subcmd_set_initial_validators)
        .get_matches();

    if matches.is_present("genkey") {
        gen_key_and_print();
    } else if let Some(m) = matches.subcommand_matches("stake") {
        let am = m.value_of("amount");
        if m.is_present("append") {
            if am.is_none() {
                println!("{}", m.usage());
            } else {
                fns::stake_append(am.unwrap()).c(d!())?;
            }
        } else {
            let cr = m.value_of("commission-rate");
            let vm = m.value_of("validator-memo");
            if am.is_none() || cr.is_none() {
                println!("{}", m.usage());
                println!(
                    "Tips: if you want to raise the power of your node, please use `fns stake --append [OPTIONS]`"
                );
            } else {
                fns::stake(am.unwrap(), cr.unwrap(), vm).c(d!())?;
            }
        }
    } else if let Some(m) = matches.subcommand_matches("unstake") {
        let am = m.value_of("amount");
        fns::unstake(am).c(d!())?;
    } else if let Some(m) = matches.subcommand_matches("claim") {
        let am = m.value_of("amount");
        fns::claim(am).c(d!())?;
    } else if matches.subcommand_matches("show").is_some() {
        fns::show().c(d!())?;
    } else if let Some(m) = matches.subcommand_matches("setup") {
        let sa = m.value_of("serv-addr");
        let om = m.value_of("owner-mnemonic-path");
        let tp = m.value_of("validator-key");
        if sa.is_none() && om.is_none() && tp.is_none() {
            println!("{}", m.usage());
        } else {
            fns::setup(sa, om, tp).c(d!())?;
        }
    } else if let Some(m) = matches.subcommand_matches("transfer") {
        let ta = m.value_of("target-addr");
        let am = m.value_of("amount");
        if ta.is_none() || am.is_none() {
            println!("{}", m.usage());
        } else {
            fns::transfer_fra(ta.unwrap(), am.unwrap()).c(d!())?;
        }
    } else if matches.is_present("set-initial-validators") {
        fns::set_initial_validators().c(d!())?;
    } else {
        println!("{}", matches.usage());
    }

    Ok(())
}

fn tip_fail(e: impl fmt::Display) {
    eprintln!("\n\x1b[31;01mFAIL !!!\x1b[00m");
    eprintln!(
        "\x1b[35;01mTips\x1b[01m:\n\tPlease send your error messages to us,\n\tif you can't understand their meanings ~^!^~\x1b[00m"
    );
    eprintln!("\n{}", e);
}

fn tip_success() {
    println!(
        "\x1b[35;01mNote\x1b[01m:\n\tYour operations has been executed without local error,\n\tbut the final result may need an asynchronous query.\x1b[00m"
    );
}

fn gen_key_and_print() {
    let mnemonic = pnk!(wallet::generate_mnemonic_custom(24, "en"));
    let key = wallet::restore_keypair_from_mnemonic_default(&mnemonic)
        .c(d!())
        .and_then(|kp| serde_json::to_string_pretty(&kp).c(d!()));
    println!(
        "\x1b[31;01mMnemonic:\x1b[00m {}\n\x1b[31;01mKey:\x1b[00m {}\n",
        mnemonic,
        pnk!(key)
    );
}
