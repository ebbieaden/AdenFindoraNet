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
//!     - "--owner-pubkey=[XfrPublicKey, base64 format]"
//!         - in a query-only environment,
//!         - you only need to set the public key
//!         - so that your private key is not exposed
//!     - "--owner-mnemonic-path=[File Path]"
//!         - the `id` of your validator will be drived from this
//! - contribute, pay some FRAs to CoinBase
//!     - "--amount=[Amout <Optional, default to '400m FRA'>]"
//! ```
//!

#![deny(warnings)]

use clap::{crate_authors, crate_version, App, SubCommand};
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
    let subcmd_stake_subcmd_append = SubCommand::with_name("append").arg_from_usage(
        "-n, --amount=[Amount] 'how much `FRA unit`s to append to your staking'",
    );
    let subcmd_stake = SubCommand::with_name("stake")
        .subcommand(subcmd_stake_subcmd_append)
        .arg_from_usage("-n, --amount=[Amount] 'how much `FRA unit`s to stake'")
        .arg_from_usage("-A, --validator-pubkey=[PubKey] 'the tendermint pubkey of your validator node'")
        .arg_from_usage("-r, --commission-rate=[Rate] 'the commission rate for your delegators")
        .arg_from_usage("-M, --validator-memo=[Memo] 'the description of your validator node'");
    let subcmd_unstake = SubCommand::with_name("unstake");
    let subcmd_claim = SubCommand::with_name("claim")
        .arg_from_usage("-n, --amount=[Amount] 'how much `FRA unit`s to claim'");
    let subcmd_show = SubCommand::with_name("show");
    let subcmd_setup = SubCommand::with_name("setup")
        .arg_from_usage(
            "-S, --serv-addr=[URL/IP] 'a fullnode address of Findora Network'",
        )
        .arg_from_usage(
            "-O, --owner-mnemonic-path=<Path>, 'Storage path of your mnemonic words'",
        )
        .arg_from_usage(
            "-k, --owner-pubkey=[PubKey], 'A `XfrPublicKey` in base64 format'",
        )
        .arg_from_usage(
            "-A, --validator-addr=[Addr], 'the tendermint address of your validator node'",
        );
    let subcmd_contribute = SubCommand::with_name("contribute").arg_from_usage(
        "-n, --amount=[Amout] 'contribute some `FRA unit`s to CoinBase'",
    );

    let matches = App::new("fns")
        .version(crate_version!())
        .author(crate_authors!())
        .about("A command line tool for staking in findora network.")
        .subcommand(subcmd_stake)
        .subcommand(subcmd_unstake)
        .subcommand(subcmd_claim)
        .subcommand(subcmd_show)
        .subcommand(subcmd_setup)
        .subcommand(subcmd_contribute)
        .get_matches();

    if let Some(m) = matches.subcommand_matches("stake") {
        if let Some(mm) = m.subcommand_matches("append") {
            let am = mm.value_of("amount");
            if am.is_none() {
                println!("{}", mm.usage());
            } else {
                fns::stake_append(am.unwrap()).c(d!())?;
            }
        } else {
            let am = m.value_of("amount");
            let va = m.value_of("validator-pubkey");
            let cr = m.value_of("commission-rate");
            let vm = m.value_of("validator-memo");
            if am.is_none() || va.is_none() || cr.is_none() {
                println!("{}", m.usage());
                println!(
                    "Tips: if you want to raise the power of your validator node, please use `fns stake append [OPTIONS]`"
                );
            } else {
                fns::stake(am.unwrap(), va.unwrap(), cr.unwrap(), vm).c(d!())?;
            }
        }
    } else if matches.subcommand_matches("unstake").is_some() {
        fns::unstake().c(d!())?;
    } else if let Some(m) = matches.subcommand_matches("claim") {
        let am = m.value_of("amount");
        fns::claim(am).c(d!())?;
    } else if matches.subcommand_matches("show").is_some() {
        fns::show().c(d!())?;
    } else if let Some(m) = matches.subcommand_matches("setup") {
        let sa = m.value_of("serv-addr");
        let om = m.value_of("owner-mnemonic-path");
        let op = m.value_of("owner-pubkey");
        let ta = m.value_of("tendermint_addr");
        if sa.is_none() && om.is_none() && op.is_none() {
            println!("{}", m.usage());
        } else {
            fns::setup(sa, om, op, ta).c(d!())?;
        }
    } else if let Some(m) = matches.subcommand_matches("contribute") {
        let am = m.value_of("amount");
        fns::contribute(am).c(d!())?;
    } else {
        println!("{}", matches.usage());
    }

    Ok(())
}

fn tip_fail(e: impl fmt::Display) {
    eprintln!("\n\x1b[31;01mFail!\x1b[00m");
    eprintln!(
        "\x1b[35;01mTips\x1b[01m:\n\tPlease send all error messages back to FindoraNetwork,\n\tif you can not understand its meaning ^!^\x1b[00m"
    );
    eprintln!("\n{}", e);
}

fn tip_success() {
    println!("\n\x1b[31;01mSuccess!\x1b[00m");
    println!(
        "\x1b[35;01mNote\x1b[01m:\n\tYour operations has been executed without local error,\n\tbut the final result may need an asynchronous query.\x1b[00m"
    );
}
