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

<<<<<<<< HEAD:src/components/finutils/src/bins/fns.rs
use clap::{crate_authors, App, Arg, ArgGroup, SubCommand};
use finutils::common;
use finutils::common::evm::*;
========
use clap::{crate_authors, App, ArgGroup, SubCommand};
use fintools::fns;
use fintools::fns::evm::*;
>>>>>>>> 42b24bd8 (merge develop refactor code (#453)):components/fintools/src/bins/fns.rs
use fp_utils::ecdsa::SecpPair;
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
    let subcmd_genkey = SubCommand::with_name("genkey")
        .about("Generate a random Findora public key/private key Pair ");
    let subcmd_stake_arggrp = ArgGroup::with_name("staking_flags")
        .args(&["commission-rate", "validator-memo"])
        .multiple(true)
        .conflicts_with("append");
    let subcmd_stake = SubCommand::with_name("stake")
        .arg_from_usage("-n, --amount=<Amount> 'how much `FRA unit`s you want to stake'")
        .arg_from_usage("-R, --commission-rate=[Rate] 'the commission rate of your node, a float number from 0.0 to 1.0'")
        .arg_from_usage("-M, --validator-memo=[Memo] 'the description of your node, optional'")
        .arg_from_usage("-a, --append 'stake more FRAs to your node'")
        .arg_from_usage("-S, --staker-priv-key=[SecretKey] 'the private key of proposer, in base64 format'")
        .arg_from_usage("-A, --validator-td-addr=[TendermintAddr] 'stake FRAs to a custom validator'")
        .about("Stake tokens (i.e. bond tokens) from a Findora account to a Validator ")
        .group(subcmd_stake_arggrp);
    let subcmd_unstake = SubCommand::with_name("unstake")
        .arg_from_usage("-S, --staker-priv-key=[SecretKey] 'the private key of proposer, in base64 format'")
        .arg_from_usage("-A, --validator-td-addr=[TendermintAddr] 'unstake FRAs from a custom validator'")
        .arg_from_usage("-n, --amount=[Amount] 'how much FRA to unstake, needed for partial undelegation'")
        .about("Unstake tokens (i.e. unbond tokens) from a Validator");
    let subcmd_claim = SubCommand::with_name("claim")
        .arg_from_usage("-n, --amount=[Amount] 'how much `FRA unit`s to claim'")
        .about("Claim accumulated FRA rewards");
    let subcmd_show = SubCommand::with_name("show")
        .about("View Validator status and accumulated rewards")
        .arg_from_usage("-b, --basic 'Show basic account info'");
    let subcmd_setup = SubCommand::with_name("setup")
        .arg_from_usage(
            "-S, --serv-addr=[URL/IP] 'a fullnode address of Findora Network'",
        )
        .arg_from_usage(
            "-O, --owner-mnemonic-path=[Path], 'storage path of your mnemonic words'",
        )
        .arg_from_usage(
            "-K, --validator-key=[Path], 'path to the tendermint keys of your validator node'",
        )
        .about("Setup environment variables for staking transactions ");
    let subcmd_transfer = SubCommand::with_name("transfer")
        .arg_from_usage(
            "-f, --from-seckey=[SecKey] 'base64-formated `XfrPrivateKey` of the receiver'",
        )
        .arg_from_usage(
            "-t, --to-pubkey=<PubKey> 'base64-formated `XfrPublicKey` of the receiver'",
        )
        .arg_from_usage("-n, --amount=<Amount> 'how much FRA units to transfer'")
        .arg_from_usage("--confidential-amount 'amounts of your TXO outputs will be confidential'")
        .arg_from_usage("--confidential-type 'asset types of your TXO outputs confidential'")
        .about("Transfer tokens from one address to another");
    //let subcmd_set_initial_validators = SubCommand::with_name("set-initial-validators");

    let subcmd_gen_eth_key = SubCommand::with_name("gen-eth-key")
        .about("Generate a random Ethereum address/private key Pair with Mnemonic");

    let subcmd_account_info = SubCommand::with_name("account")
        .about("Return user contract account information")
        .arg_from_usage(
            "-a --address=[Address] 'Findora account(eg:fra1rkv...) or Ethereum address(g:0xd3Bf...)'",
        );

    let subcmd_deposit = SubCommand::with_name("contract-deposit")
        .about("Transfer FRA from a Findora account to the specified Ethereum address")
        .arg_from_usage(
            "-a --address=[Address] 'Ethereum address to receive FRA, eg:0xd3Bf...'",
        )
        .arg_from_usage("-b --balance=<Balance> 'Deposit FRA amount'");

    let subcmd_withdraw = SubCommand::with_name("contract-withdraw")
        .about(
            "Transfer FRA from an Ethereum address to the specified Findora account",
        )
        .arg_from_usage(
            "-a --address=[Address] 'Findora account to receive FRA, eg:fra1rkv...'",
        )
        .arg_from_usage("-b --balance=<Balance> 'Withdraw FRA amount'")
        .arg_from_usage(
            "-e --eth-key=[MNEMONIC] 'Ethereum account mnemonic phrase sign withdraw tx'",
        );

    let matches = App::new("fns")
        .version(common::version())
        .author(crate_authors!())
        .about("A command line tool for staking in findora network.")
        .arg_from_usage("-v, --version")
        .subcommand(subcmd_genkey)
        .subcommand(subcmd_stake)
        .subcommand(subcmd_unstake)
        .subcommand(subcmd_claim)
        .subcommand(subcmd_show)
        .subcommand(subcmd_setup)
        .subcommand(subcmd_transfer)
        .subcommand(subcmd_gen_eth_key)
        .subcommand(subcmd_account_info)
        .subcommand(subcmd_deposit)
        .subcommand(subcmd_withdraw)
        //.subcommand(subcmd_set_initial_validators)
        .get_matches();

    if matches.is_present("version") {
        println!("{}", env!("VERGEN_SHA"));
    } else if matches.is_present("genkey") {
<<<<<<<< HEAD:src/components/finutils/src/bins/fns.rs
        common::gen_key_and_print();
    } else if let Some(m) = matches.subcommand_matches("staker-update") {
        let cr = m.value_of("commission-rate");
        let vm = m.value_of("validator-memo");
        if vm.is_none() && cr.is_none() {
            println!("{}", m.usage());
            println!(
                "Tips: to update the information of your node, please specify commission-rate or memo"
            );
        } else {
            common::staker_update(cr, vm).c(d!())?;
        }
========
        gen_key_and_print();
>>>>>>>> 42b24bd8 (merge develop refactor code (#453)):components/fintools/src/bins/fns.rs
    } else if let Some(m) = matches.subcommand_matches("stake") {
        let am = m.value_of("amount");
        if m.is_present("append") {
            let staker = m.value_of("staker-priv-key");
            let td_addr = m.value_of("validator-td-addr");
            if am.is_none() {
                println!("{}", m.usage());
            } else {
                common::stake_append(am.unwrap(), staker, td_addr).c(d!())?;
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
<<<<<<<< HEAD:src/components/finutils/src/bins/fns.rs
                common::stake(am.unwrap(), cr.unwrap(), vm, force).c(d!())?;
========
                fns::stake(am.unwrap(), cr.unwrap(), vm).c(d!())?;
>>>>>>>> 42b24bd8 (merge develop refactor code (#453)):components/fintools/src/bins/fns.rs
            }
        }
    } else if let Some(m) = matches.subcommand_matches("unstake") {
        let am = m.value_of("amount");
        let staker = m.value_of("staker-priv-key");
        let td_addr = m.value_of("validator-td-addr");
        common::unstake(am, staker, td_addr).c(d!())?;
    } else if let Some(m) = matches.subcommand_matches("claim") {
        let am = m.value_of("amount");
        common::claim(am).c(d!())?;
    } else if let Some(m) = matches.subcommand_matches("show") {
        let basic = m.is_present("basic");
        common::show(basic).c(d!())?;
    } else if let Some(m) = matches.subcommand_matches("setup") {
        let sa = m.value_of("serv-addr");
        let om = m.value_of("owner-mnemonic-path");
        let tp = m.value_of("validator-key");
        if sa.is_none() && om.is_none() && tp.is_none() {
            println!("{}", m.usage());
        } else {
            common::setup(sa, om, tp).c(d!())?;
        }
    } else if let Some(m) = matches.subcommand_matches("transfer") {
        let f = m.value_of("from-seckey");
<<<<<<<< HEAD:src/components/finutils/src/bins/fns.rs
        let t = m
            .value_of("to-pubkey")
            .map(|pk_str| pk_str.to_owned())
            .c(d!())
            .or_else(|_| {
                m.value_of("to-wallet-address")
                    .c(d!())
                    .and_then(|addr| {
                        libutils::wallet::public_key_from_bech32(addr)
                            .c(d!("invalid wallet address"))
                    })
                    .map(|pk| libutils::wallet::public_key_to_base64(&pk))
            })?;
========
        let t = m.value_of("to-pubkey");
>>>>>>>> 42b24bd8 (merge develop refactor code (#453)):components/fintools/src/bins/fns.rs
        let am = m.value_of("amount");

        if t.is_none() || am.is_none() {
            println!("{}", m.usage());
        } else {
            common::transfer_fra(
                f,
                t.unwrap(),
                am.unwrap(),
                m.is_present("confidential-amount"),
                m.is_present("confidential-type"),
            )
            .c(d!())?;
        }
    } else if matches.is_present("set-initial-validators") {
        common::set_initial_validators().c(d!())?;
    } else if matches.is_present("gen-eth-key") {
        let (pair, phrase, _) = SecpPair::generate_with_phrase(None);
        let kp = hex::encode(pair.seed());
        println!(
            "\x1b[31;01mMnemonic:\x1b[00m {}\n\x1b[31;01mPrivateKey:\x1b[00m {}\n\x1b[31;01mAddress:\x1b[00m {:?}\n",
            phrase,
            kp,
            pair.address()
        );
    } else if let Some(m) = matches.subcommand_matches("account") {
        let address = m.value_of("address");
        let (account, info) = contract_account_info(address)?;
        println!("AccountId: {}\n{:#?}\n", account, info);
    } else if let Some(m) = matches.subcommand_matches("contract-deposit") {
        let amount = m.value_of("balance").c(d!())?;
        let address = m.value_of("address");
        transfer_to_account(u64::from_str_radix(amount, 10).c(d!())?, address)?
    } else if let Some(m) = matches.subcommand_matches("contract-withdraw") {
        let amount = m.value_of("balance").c(d!())?;
        let address = m.value_of("address");
        let eth_key = m.value_of("eth-key");
        transfer_from_account(
            u64::from_str_radix(amount, 10).c(d!())?,
            address,
            eth_key,
        )?
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
    let (m, k) = loop {
        let mnemonic = pnk!(wallet::generate_mnemonic_custom(24, "en"));
        let key = wallet::restore_keypair_from_mnemonic_default(&mnemonic)
            .c(d!())
            .and_then(|kp| serde_json::to_string_pretty(&kp).c(d!()));
        let k = pnk!(key);
        if !k.contains('-') {
            break (mnemonic, k);
        }
    };
    println!(
        "\x1b[31;01mMnemonic:\x1b[00m {}\n\x1b[31;01mKey:\x1b[00m {}\n",
        m, k
    );
}
