//!
//! # Findora Network Staking
//!
//! FNS, a command line tool for staking in findora network.
//!
//! This module is the library part of FNS.
//!

use crate::fns::utils::parse_td_validator_keys;
use lazy_static::lazy_static;
use ledger::{
    data_model::BLACK_HOLE_PUBKEY_STAKING,
    staking::{
        check_delegation_amount, gen_random_keypair, td_pubkey_to_td_addr,
        td_pubkey_to_td_addr_bytes, PartialUnDelegation,
    },
};
use ruc::*;
use std::fs;
use tendermint::PrivateKey;
use txn_builder::BuildsTransactions;
use zei::xfr::sig::XfrKeyPair;

pub mod utils;

const CFG_PATH: &str = "/tmp/.____fns_config____";

lazy_static! {
    static ref MNEMONIC: Option<String> = fs::read_to_string(&*MNEMONIC_FILE).ok();
    static ref MNEMONIC_FILE: String = format!("{}/mnemonic", CFG_PATH);
    static ref TD_KEY: Option<String> = fs::read_to_string(&*TD_KEY_FILE).ok();
    static ref TD_KEY_FILE: String = format!("{}/tendermint_keys", CFG_PATH);
    static ref SERV_ADDR: Option<String> = fs::read_to_string(&*SERV_ADDR_FILE).ok();
    static ref SERV_ADDR_FILE: String = format!("{}/serv_addr", CFG_PATH);
}

pub fn stake(amount: &str, commission_rate: &str, memo: Option<&str>) -> Result<()> {
    let am = amount.parse::<u64>().c(d!("'amount' must be an integer"))?;
    check_delegation_amount(am).c(d!())?;
    let cr = commission_rate
        .parse::<f64>()
        .c(d!("commission rate must be a float number"))
        .and_then(|cr| convert_commission_rate(cr).c(d!()))?;
    let td_pubkey = get_td_pubkey().c(d!())?;

    let kp = get_keypair().c(d!())?;
    let vkp = get_td_privkey().c(d!())?;

    let mut builder = utils::new_tx_builder().c(d!())?;
    builder
        .add_operation_staking(&kp, &vkp, td_pubkey, cr, memo.map(|m| m.to_owned()))
        .c(d!())?;
    utils::gen_transfer_op(&kp, vec![(&BLACK_HOLE_PUBKEY_STAKING, am)])
        .c(d!())
        .map(|principal_op| builder.add_operation(principal_op))?;

    utils::send_tx(&builder.take_transaction()).c(d!())
}

pub fn stake_append(amount: &str) -> Result<()> {
    let am = amount.parse::<u64>().c(d!("'amount' must be an integer"))?;
    check_delegation_amount(am).c(d!())?;

    let td_pubkey = get_td_pubkey().c(d!())?;
    let td_addr = td_pubkey_to_td_addr(&td_pubkey);

    let kp = get_keypair().c(d!())?;

    let mut builder = utils::new_tx_builder().c(d!())?;
    builder.add_operation_delegation(&kp, td_addr);
    utils::gen_transfer_op(&kp, vec![(&BLACK_HOLE_PUBKEY_STAKING, am)])
        .c(d!())
        .map(|principal_op| builder.add_operation(principal_op))?;

    utils::send_tx(&builder.take_transaction()).c(d!())
}

pub fn unstake(am: Option<&str>) -> Result<()> {
    let am = if let Some(i) = am {
        Some(i.parse::<u64>().c(d!("'amount' must be an integer"))?)
    } else {
        None
    };

    let kp = get_keypair().c(d!())?;
    let td_addr_bytes = get_td_pubkey()
        .c(d!())
        .map(|td_pk| td_pubkey_to_td_addr_bytes(&td_pk))?;

    let mut builder = utils::new_tx_builder().c(d!())?;

    utils::gen_fee_op(&kp).c(d!()).map(|op| {
        builder.add_operation(op);
        if let Some(am) = am {
            // partial undelegation
            builder.add_operation_undelegation(
                &kp,
                Some(PartialUnDelegation::new(
                    am,
                    gen_random_keypair().get_pk(),
                    td_addr_bytes,
                )),
            );
        } else {
            builder.add_operation_undelegation(&kp, None);
        }
    })?;

    utils::send_tx(&builder.take_transaction()).c(d!())
}

pub fn claim(am: Option<&str>) -> Result<()> {
    let am = if let Some(i) = am {
        Some(i.parse::<u64>().c(d!("'amount' must be an integer"))?)
    } else {
        None
    };

    let kp = get_keypair().c(d!())?;
    let mut builder = utils::new_tx_builder().c(d!())?;

    utils::gen_fee_op(&kp).c(d!()).map(|op| {
        builder.add_operation(op);
        builder.add_operation_claim(&kp, am);
    })?;

    utils::send_tx(&builder.take_transaction()).c(d!())
}

pub fn show() -> Result<()> {
    let kp = get_keypair().c(d!())?;

    let serv_addr = ruc::info!(get_serv_addr()).map(|i| {
        println!("\x1b[31;01mServer URL:\x1b[00m\n{}\n", i);
    });

    let xfr_pubkey = ruc::info!(get_keypair()).map(|i| {
        println!(
            "\x1b[31;01mXfrPublicKey:\x1b[00m\n{}\n",
            wallet::public_key_to_base64(&i.get_pk())
        );
    });

    let td_pubkey = ruc::info!(get_td_pubkey()).map(|i| {
        println!(
            "\x1b[31;01mValidator Node Addr:\x1b[00m\n{}\n",
            td_pubkey_to_td_addr(&i)
        );
    });

    let self_balance = ruc::info!(utils::get_balance(&kp)).map(|i| {
        println!("\x1b[31;01mYour Balance:\x1b[00m\n{} FRA units\n", i);
    });

    let delegation_info = utils::get_delegation_info(kp.get_pk_ref())
        .c(d!("fail to connect to server"))
        .and_then(|di| {
            serde_json::to_string_pretty(&di).c(d!("server returned invalid data"))
        });
    let delegation_info = ruc::info!(delegation_info).map(|i| {
        println!("\x1b[31;01mYour Staking:\x1b[00m\n{}\n", i);
    });

    if [
        serv_addr,
        xfr_pubkey,
        td_pubkey,
        self_balance,
        delegation_info,
    ]
    .iter()
    .any(|i| i.is_err())
    {
        Err(eg!("unable to obtain complete information"))
    } else {
        Ok(())
    }
}

pub fn setup(
    serv_addr: Option<&str>,
    owner_mnemonic_path: Option<&str>,
    validator_key_path: Option<&str>,
) -> Result<()> {
    fs::create_dir_all(CFG_PATH).c(d!("fail to create config path"))?;

    if let Some(sa) = serv_addr {
        fs::write(&*SERV_ADDR_FILE, sa).c(d!("fail to cache 'serv-addr'"))?;
    }
    if let Some(mp) = owner_mnemonic_path {
        fs::write(&*MNEMONIC_FILE, mp).c(d!("fail to cache 'owner-mnemonic-path'"))?;
    }
    if let Some(kp) = validator_key_path {
        fs::write(&*TD_KEY_FILE, kp).c(d!("fail to cache 'validator-key-path'"))?;
    }
    Ok(())
}

pub fn transfer_fra(target_addr: &str, am: &str) -> Result<()> {
    let ta =
        wallet::public_key_from_base64(target_addr).c(d!("invalid 'target-addr'"))?;
    let am = am.parse::<u64>().c(d!("'amount' must be an integer"))?;

    get_keypair()
        .c(d!())
        .and_then(|kp| utils::transfer(&kp, &ta, am).c(d!()))
}

/// Mainly for official usage,
/// and can be also used in test scenes.
pub fn set_initial_validators() -> Result<()> {
    get_keypair()
        .c(d!())
        .and_then(|kp| utils::set_initial_validators(&kp).c(d!()))
}

fn get_serv_addr() -> Result<&'static str> {
    if let Some(sa) = SERV_ADDR.as_ref() {
        Ok(sa)
    } else {
        Err(eg!("'serv-addr' has not been set"))
    }
}

fn get_keypair() -> Result<XfrKeyPair> {
    if let Some(m_path) = MNEMONIC.as_ref() {
        fs::read_to_string(m_path)
            .c(d!("can not read mnemonic from 'owner-mnemonic-path'"))
            .and_then(|m| {
                wallet::restore_keypair_from_mnemonic_default(m.trim())
                    .c(d!("invalid 'owner-mnemonic'"))
            })
    } else {
        Err(eg!("'owner-mnemonic-path' has not been set"))
    }
}

fn get_td_pubkey() -> Result<Vec<u8>> {
    if let Some(key_path) = TD_KEY.as_ref() {
        fs::read_to_string(key_path)
            .c(d!("can not read key file from path"))
            .and_then(|k| {
                let v_keys = parse_td_validator_keys(k).c(d!())?;
                Ok(v_keys.pub_key.to_vec())
            })
    } else {
        Err(eg!("'validator-pubkey' has not been set"))
    }
}

fn get_td_privkey() -> Result<PrivateKey> {
    if let Some(key_path) = TD_KEY.as_ref() {
        fs::read_to_string(key_path)
            .c(d!("can not read key file from path"))
            .and_then(|k| {
                parse_td_validator_keys(k)
                    .c(d!())
                    .map(|v_keys| v_keys.priv_key)
            })
    } else {
        Err(eg!("'validator-privkey' has not been set"))
    }
}

fn convert_commission_rate(cr: f64) -> Result<[u64; 2]> {
    if 1.0 < cr {
        return Err(eg!("commission rate can exceed 100%"));
    }
    if 0.0 > cr {
        return Err(eg!("commission rate must be a positive float number"));
    }
    Ok([(cr * 10000.0) as u64, 10000])
}
