//!
//! # staking_tester
//!
//! - init --server-addr=<127.0.0.1> --server-port=<8669>
//! - delegate --user=<cat1> --amount=<N> --validator=<dog1>
//! - undelegate --user=<cat1>
//! - claim --user=<cat1> --amount=<N>
//!

#![deny(warnings)]

use clap::{crate_authors, crate_version, App, SubCommand};
use lazy_static::lazy_static;
use ledger::{
    data_model::{
        DelegationInfo, Operation, StateCommitmentData, Transaction, TransferType,
        TxoRef, TxoSID, Utxo, ASSET_TYPE_FRA, BLACK_HOLE_PUBKEY, TX_FEE_MIN,
    },
    staking::{check_delegation_amount, COINBASE_PK, COINBASE_PRINCIPAL_PK},
    store::fra_gen_initial_tx,
};
use ruc::*;
use serde::Serialize;
use std::{collections::BTreeMap, env};
use txn_builder::{BuildsTransactions, TransactionBuilder, TransferOperationBuilder};
use utils::{HashOf, SignatureOf};
use zei::xfr::{
    asset_record::{open_blind_asset_record, AssetRecordType},
    sig::{XfrKeyPair, XfrPublicKey},
    structs::{AssetRecordTemplate, XfrAmount},
};

lazy_static! {
    static ref SERV_ADDR: String =
        env::var("STAKING_TESTER_SERV_ADDR").unwrap_or_else(|_| "localhost".to_owned());
    static ref USER_LIST: BTreeMap<Name, User> = gen_user_list();
    static ref VALIDATOR_LIST: BTreeMap<Name, Validator> = gen_valiator_list();
}

const ROOT_MNEMONIC: &str = "bright poem guard trade airport artist soon mountain shoe satisfy fox adapt garden decline uncover when pilot person flat bench connect coach planet hidden";

type Name = String;
type NameRef<'a> = &'a str;

fn main() {
    pnk!(run());
}

fn run() -> Result<()> {
    let subcmd_init = SubCommand::with_name("init");
    let subcmd_delegate = SubCommand::with_name("delegate")
        .arg_from_usage("-u, --user=[User] 'user name of delegator'")
        .arg_from_usage("-n, --amount=[Amount] 'how much FRA to delegate'")
        .arg_from_usage("-v, --validator=[Validator] 'which validator to delegate to'");
    let subcmd_undelegate = SubCommand::with_name("undelegate")
        .arg_from_usage("-u, --user=[User] 'user name of delegator'");
    let subcmd_claim = SubCommand::with_name("claim")
        .arg_from_usage("-u, --user=[User] 'user name of delegator'")
        .arg_from_usage("-n, --amount=[Amount] 'how much FRA to delegate'");
    let subcmd_show = SubCommand::with_name("show")
        .arg_from_usage("-b, --coinbase 'show the infomation about coinbase'")
        .arg_from_usage("-r, --root-mnemonic 'show the pre-defined root mnemonic'")
        .arg_from_usage("-U, --user-list 'show the pre-defined user list'")
        .arg_from_usage("-v, --validator-list 'show the pre-defined validator list'")
        .arg_from_usage("-u, --user=[User] 'user name of delegator'");

    let matches = App::new("stt")
        .version(crate_version!())
        .author(crate_authors!())
        .about("A manual test tool for the staking function.")
        .subcommand(subcmd_init)
        .subcommand(subcmd_delegate)
        .subcommand(subcmd_undelegate)
        .subcommand(subcmd_claim)
        .subcommand(subcmd_show)
        .get_matches();

    if matches.subcommand_matches("init").is_some() {
        init::init().c(d!())?;
    } else if let Some(m) = matches.subcommand_matches("delegate") {
        let user = m.value_of("user");
        let amount = m.value_of("amount");
        let validator = m.value_of("validator");

        if user.is_none() || amount.is_none() || validator.is_none() {
            println!("{}", m.usage());
        } else {
            let amount = amount.unwrap().parse::<u64>().c(d!())?;
            delegate::gen_tx(user.unwrap(), amount, validator.unwrap())
                .c(d!())
                .and_then(|tx| send_tx(&tx).c(d!()))?;
        }
    } else if let Some(m) = matches.subcommand_matches("undelegate") {
        let user = m.value_of("user");

        if user.is_none() {
            println!("{}", m.usage());
        } else {
            undelegate::gen_tx(user.unwrap())
                .c(d!())
                .and_then(|tx| send_tx(&tx).c(d!()))?;
        }
    } else if let Some(m) = matches.subcommand_matches("claim") {
        let user = m.value_of("user");
        let amount = m.value_of("amount");

        if user.is_none() || amount.is_none() {
            println!("{}", m.usage());
        } else {
            let amount = amount.unwrap().parse::<u64>().c(d!())?;
            claim::gen_tx(user.unwrap(), amount)
                .c(d!())
                .and_then(|tx| send_tx(&tx).c(d!()))?;
        }
    } else if let Some(m) = matches.subcommand_matches("show") {
        let cb = m.is_present("coinbase");
        let rm = m.is_present("root-mnemonic");
        let ul = m.is_present("user-list");
        let vl = m.is_present("validator-list");
        let u = m.value_of("user");

        if cb || rm || ul || vl || u.is_some() {
            print_info(cb, rm, ul, vl, u).c(d!())?;
        } else {
            println!("{}", m.usage());
        }
    } else {
        println!("{}", matches.usage());
    }

    Ok(())
}

mod init {
    use super::*;

    pub fn init() -> Result<()> {
        let root_kp =
            wallet::restore_keypair_from_mnemonic_default(ROOT_MNEMONIC).c(d!())?;

        send_tx(&fra_gen_initial_tx(&root_kp)).c(d!())?;

        sleep_ms!(10 * 1000);

        let mut target_list = USER_LIST
            .values()
            .map(|u| &u.pubkey)
            .chain(VALIDATOR_LIST.values().map(|v| &v.pubkey))
            .map(|pk| (pk, 2_000_000_000_000))
            .collect::<Vec<_>>();

        target_list.push((&*COINBASE_PK, 4_000_000_000_000));

        transfer(&root_kp, target_list).c(d!())?;

        sleep_ms!(10 * 1000);

        for v in VALIDATOR_LIST.values() {
            delegate::gen_tx(&v.name, 1_000_000_000_000, &v.name)
                .c(d!())
                .and_then(|tx| send_tx(&tx).c(d!()))?;
        }

        Ok(())
    }
}

mod delegate {
    use super::*;

    pub fn gen_tx(
        user: NameRef,
        amount: u64,
        validator: NameRef,
    ) -> Result<Transaction> {
        check_delegation_amount(amount).c(d!())?;

        let owner_kp = USER_LIST
            .get(user)
            .map(|u| &u.keypair)
            .or_else(|| VALIDATOR_LIST.get(user).map(|v| &v.keypair))
            .c(d!())?;
        let validator = &VALIDATOR_LIST.get(validator).c(d!())?.td_addr;

        let mut builder = new_tx_builder().c(d!())?;
        builder.add_operation_delegation(owner_kp, validator.to_owned());

        let trans_to_self =
            gen_transfer_op(owner_kp, vec![(&COINBASE_PRINCIPAL_PK, amount)], false)
                .c(d!())?;
        builder.add_operation(trans_to_self);

        if builder.add_fee_relative_auto(&owner_kp).is_err() {
            builder.add_operation(gen_fee_op(owner_kp).c(d!())?);
        }

        Ok(builder.take_transaction())
    }
}

mod undelegate {
    use super::*;

    pub fn gen_tx(user: NameRef) -> Result<Transaction> {
        let owner_kp = &USER_LIST.get(user).c(d!())?.keypair;

        let mut builder = new_tx_builder().c(d!())?;
        builder.add_operation_undelegation(owner_kp);

        gen_fee_op(owner_kp)
            .c(d!())
            .map(|op| builder.add_operation(op))?;

        Ok(builder.take_transaction())
    }
}

mod claim {
    use super::*;

    pub fn gen_tx(user: NameRef, amount: u64) -> Result<Transaction> {
        let owner_kp = &USER_LIST.get(user).c(d!())?.keypair;

        let mut builder = new_tx_builder().c(d!())?;
        builder.add_operation_claim(owner_kp, amount);

        gen_fee_op(owner_kp)
            .c(d!())
            .map(|op| builder.add_operation(op))?;

        Ok(builder.take_transaction())
    }
}

fn print_info(
    show_coinbse: bool,
    show_root_mnemonic: bool,
    show_user_list: bool,
    show_validator_list: bool,
    user: Option<NameRef>,
) -> Result<()> {
    if show_coinbse {
        let cb_balance = get_balance_x(&COINBASE_PK).c(d!())?;
        let cb_principal_balance = get_balance_x(&COINBASE_PRINCIPAL_PK).c(d!())?;

        println!(
            "\x1b[31;01mCOINBASE BALANCE:\x1b[00m\n{} FRA units\n",
            cb_balance
        );
        println!(
            "\x1b[31;01mCOINBASE PRINCIPAL BALANCE:\x1b[00m\n{} FRA units\n",
            cb_principal_balance
        );
    }

    if show_root_mnemonic {
        println!("\x1b[31;01mROOT MNEMONIC:\x1b[00m\n{}\n", ROOT_MNEMONIC);
    }

    if show_user_list {
        let user_list = serde_json::to_string_pretty(&*USER_LIST).c(d!())?;
        println!("\x1b[31;01mUSER LIST:\x1b[00m\n{}\n", user_list);
    }

    if show_validator_list {
        let validator_list = serde_json::to_string_pretty(&*VALIDATOR_LIST).c(d!())?;
        println!("\x1b[31;01mVALIDATOR LIST:\x1b[00m\n{}\n", validator_list);
    }

    if let Some(u) = user {
        let balance = get_balance(u).c(d!())?;
        println!("\x1b[31;01mUSER BALANCE:\x1b[00m\n{} FRA units\n", balance);

        let user_info = get_delegation_info(u).c(d!())?;
        println!("\x1b[31;01mUSER DELEGATION:\x1b[00m\n{}\n", user_info);
    }

    Ok(())
}

fn send_tx(tx: &Transaction) -> Result<()> {
    let url = format!("http://{}:8669/submit_transaction", &*SERV_ADDR);
    attohttpc::post(&url)
        .header(attohttpc::header::CONTENT_TYPE, "application/json")
        .bytes(&serde_json::to_vec(tx).c(d!())?)
        .send()
        .c(d!())
        .map(|_| ())
}

fn new_tx_builder() -> Result<TransactionBuilder> {
    get_seq_id().c(d!()).map(TransactionBuilder::from_seq_id)
}

fn get_seq_id() -> Result<u64> {
    type Resp = (
        HashOf<Option<StateCommitmentData>>,
        u64,
        SignatureOf<(HashOf<Option<StateCommitmentData>>, u64)>,
    );

    let url = format!("http://{}:8668/global_state", &*SERV_ADDR);

    attohttpc::get(&url)
        .send()
        .c(d!())?
        .error_for_status()
        .c(d!())?
        .bytes()
        .c(d!())
        .and_then(|b| serde_json::from_slice::<Resp>(&b).c(d!()))
        .map(|resp| resp.1)
}

fn get_delegation_info(user: NameRef) -> Result<String> {
    let pk = USER_LIST
        .get(user)
        .map(|u| &u.pubkey)
        .or_else(|| VALIDATOR_LIST.get(user).map(|v| &v.pubkey))
        .c(d!())?;

    let url = format!(
        "http://{}:8668/delegation_info/{}",
        &*SERV_ADDR,
        wallet::public_key_to_base64(pk)
    );

    attohttpc::get(&url)
        .send()
        .c(d!())?
        .error_for_status()
        .c(d!())?
        .bytes()
        .c(d!())
        .and_then(|b| serde_json::from_slice::<DelegationInfo>(&b).c(d!()))
        .and_then(|resp| serde_json::to_string_pretty(&resp).c(d!()))
}

fn get_balance(user: NameRef) -> Result<u64> {
    let pk = USER_LIST
        .get(user)
        .map(|u| &u.pubkey)
        .or_else(|| VALIDATOR_LIST.get(user).map(|v| &v.pubkey))
        .c(d!())?;

    get_balance_x(pk).c(d!())
}

fn get_balance_x(pk: &XfrPublicKey) -> Result<u64> {
    let balance = get_owned_utxos(pk)
        .c(d!())?
        .values()
        .map(|utxo| {
            if let XfrAmount::NonConfidential(am) = utxo.0.record.amount {
                am
            } else {
                0
            }
        })
        .sum();

    Ok(balance)
}

fn get_owned_utxos(addr: &XfrPublicKey) -> Result<BTreeMap<TxoSID, Utxo>> {
    let url = format!(
        "http://{}:8668/owned_utxos/{}",
        &*SERV_ADDR,
        wallet::public_key_to_base64(addr)
    );
    attohttpc::get(&url)
        .send()
        .c(d!())?
        .error_for_status()
        .c(d!())?
        .bytes()
        .c(d!())
        .and_then(|b| serde_json::from_slice(&b).c(d!()))
}

#[derive(Debug, Serialize)]
struct User {
    name: String,
    mnemonic: String,
    pubkey: XfrPublicKey,
    keypair: XfrKeyPair,
}

fn gen_user_list() -> BTreeMap<Name, User> {
    const MNEMONIC_LIST: [&str; 5] = [
        "bunker boring twenty addict element cover owner economy catalog cause staff shock say wave rent submit clean cinnamon visit erase rescue transfer wave forget",
        "swap mail library enrich flee strike property flock unhappy betray bitter awake health glimpse armed good tip bicycle skill belt beyond smooth flush ring",
        "job latin tilt burden address grid opinion lazy mystery crystal pink pen lady public fall magnet method pact pill frost champion symptom zero problem",
        "ostrich pill knee divorce situate firm size dilemma cushion broccoli evolve carbon start virtual cave ask hat until physical nothing flash bunker inject thrive",
        "priority venue mail camp lens myself media base head fringe endorse amazing flower winter danger mammal walnut fabric please letter access suspect shed country",
    ];

    (0..MNEMONIC_LIST.len())
        .map(|i| {
            let keypair = pnk!(wallet::restore_keypair_from_mnemonic_default(
                MNEMONIC_LIST[i]
            ));
            let pubkey = keypair.get_pk();
            User {
                name: format!("u{}", 1 + i),
                mnemonic: MNEMONIC_LIST[i].to_owned(),
                pubkey,
                keypair,
            }
        })
        .map(|u| (u.name.clone(), u))
        .collect()
}

#[derive(Debug, Serialize)]
struct Validator {
    name: String,
    td_addr: String,
    pubkey: XfrPublicKey,
    keypair: XfrKeyPair,
}

fn gen_valiator_list() -> BTreeMap<Name, Validator> {
    const NUM: usize = 20;
    const TD_ADDR_LIST: [&str; NUM] = [
        "611C922247C3BE7EA13455B191B6EFD909F10196",
        "5A006EA8455C6DB35B4B60B7218774B2E589482B",
        "0F64C8259BFCD1A9F6E21958D0A60D9E370D9C13",
        "A9534BB329FE980838EC0FEB7550AD66228D581B",
        "7DEFDDA9E24A1C4320A9D45B8E7F14A40E479713",
        "4C2582DC314575DE73AD1EAA06726E555786900E",
        "82DEBD3B6C108095BDD3FE7352B9C538BDEFA621",
        "EC046D54F2FA16AE7126343425C1E91A96ED18BD",
        "325EC027285ABAA2A755286E1982E8F66633C05B",
        "CF7D19D604FF5EFE7EC90583D5700D7FF1CF63BA",
        "30E07994969FFE8007481914335521CE665BEEFE",
        "59A3EC547FCFA2434F64A09F0B85A9BB6262F71B",
        "88C045F586A338E90CE9A712FC4F13D04764E28F",
        "91F40F5F761DF9A09D9CA7E6200D02551BBA31F1",
        "57AF4341DE9A2A3725123718DEDBA5C7B9141E7D",
        "908D050231F5D568DB11F379DC5B3E8A7C8A453D",
        "D88C6FE77A7F3F84578D6D9AA2718BB034743902",
        "55B8CF069F6F6C75935F8EB5FAC6B8C8138BC954",
        "8424784D8505B2661F120831D18BE0021DD0CDA8",
        "9F832EE81DB4FBDAA8D3541ECA6ECEE0E97C119B",
    ];
    const MNEMONIC_LIST: [&str; NUM] = [
        "alien pride power ostrich will cart crumble judge ordinary picnic bring dinner nut success phone banana fold agent shallow silent dose feel short insane",
        "erode wasp helmet advice olive ridge update kid drip toast agree hand oppose hurt creek hazard purity raise main organ bargain patrol ramp toward",
        "eternal happy outer ankle write smile admit scrub disease know code mom juice rapid blast ensure switch settle news antique into conduct sphere empower",
        "script month grain cook demise student foam odor boring endorse layer spell force culture height style observe husband embody tiger that athlete genius clap",
        "sustain walk alley since scheme age glue choice fat current frog swallow cable company arrive receive parade anger illness clean maple draft art exile",
        "state sick tip glare erupt sign salad melt library churn accident organ book trust sketch embrace addict ice always trouble original vendor merge monkey",
        "vague random rule forum moon page opinion alcohol mixed circle ask cost life history vast garden reunion use flame west nothing middle kangaroo language",
        "peace patrol canvas regular together cycle clown region carpet learn price plate state gate long rose topple mango auto canoe media cushion soccer argue",
        "clump guard become smoke satisfy recall nation oil slide shell case notable escape suspect dawn poverty report smile apology learn column jelly fiber outer",
        "element update essence melody evolve razor canvas alcohol destroy tank neutral ride coast dish april cup medal brave palm strike essay flower learn what",
        "firm when photo pupil cream design pulse script mule among pupil cloth mechanic obvious amazing panic broom indoor silly member purpose rather upgrade hover",
        "canvas put chalk network thunder caught pigeon voyage dune despair ability hour light between lawsuit breeze disorder naive surround marine ostrich grace report galaxy",
        "account peasant found dignity thumb about taste yard elbow truth journey night model cushion dirt suit skirt bus flat dwarf across noble need between",
        "federal day velvet stairs liberty burst pluck margin capable subway rail eye where spread video journey garden trap salmon sword industry shine elephant arena",
        "empty shy abandon elegant case outside drift voice tuition grace slush vibrant wage future script split educate insect involve unusual method arena option add",
        "theme light sun cram fluid lab entire edge iron visa salt father stomach buffalo keep helmet sword sure shy shop flight teach diary brand",
        "comfort elephant manual blur climb blue disagree skate ridge auction loyal remember obscure nurse bar insane please refuse rather once giant fiber midnight foil",
        "choice speed eternal movie glide culture deal elite sick aspect cluster cruel net moment myself jeans fade radio reflect desk grit toast this proof",
        "strong fever clock wear forum palm celery smart sting mesh barrel again drive note clump cross unfold buddy tube lesson future lounge flat dune",
        "margin mention suit twice submit horse drive myth afraid upper neither reward refuse cart caught nurse era beef exclude goose large borrow mansion universe",
    ];

    (0..NUM)
        .map(|i| {
            let td_addr = TD_ADDR_LIST[i].to_owned();
            let keypair = pnk!(wallet::restore_keypair_from_mnemonic_default(
                MNEMONIC_LIST[i]
            ));
            let pubkey = keypair.get_pk();
            Validator {
                name: format!("v{}", 1 + i),
                td_addr,
                pubkey,
                keypair,
            }
        })
        .map(|v| (v.name.clone(), v))
        .collect()
}

fn transfer(
    owner_kp: &XfrKeyPair,
    target_list: Vec<(&XfrPublicKey, u64)>,
) -> Result<()> {
    let mut builder = new_tx_builder().c(d!())?;
    builder.add_operation(gen_transfer_op(owner_kp, target_list, false).c(d!())?);

    if builder.add_fee_relative_auto(&owner_kp).is_err() {
        builder.add_operation(gen_fee_op(owner_kp).c(d!())?);
    }

    send_tx(&builder.take_transaction()).c(d!())
}

fn gen_fee_op(owner_kp: &XfrKeyPair) -> Result<Operation> {
    gen_transfer_op(owner_kp, vec![(&*BLACK_HOLE_PUBKEY, TX_FEE_MIN)], true).c(d!())
}

fn gen_transfer_op(
    owner_kp: &XfrKeyPair,
    target_list: Vec<(&XfrPublicKey, u64)>,
    rev: bool,
) -> Result<Operation> {
    let mut trans_builder = TransferOperationBuilder::new();

    let mut am = target_list.iter().map(|(_, am)| *am).sum();
    let mut i_am;
    let utxos = get_owned_utxos(owner_kp.get_pk_ref()).c(d!())?.into_iter();

    macro_rules! add_inputs {
        ($utxos: expr) => {
            for (sid, utxo) in $utxos {
                if let XfrAmount::NonConfidential(n) = utxo.0.record.amount {
                    alt!(n < am, i_am = n, i_am = am);
                    am = am.saturating_sub(n);
                } else {
                    continue;
                }

                open_blind_asset_record(&utxo.0.record, &None, owner_kp)
                    .c(d!())
                    .and_then(|ob| {
                        trans_builder
                            .add_input(TxoRef::Absolute(sid), ob, None, None, i_am)
                            .c(d!())
                    })?;
                alt!(0 == am, break);
            }
        };
    }

    alt!(rev, add_inputs!(utxos.rev()), add_inputs!(utxos));

    if 0 != am {
        return Err(eg!());
    }

    let outputs = target_list.into_iter().map(|(pk, n)| {
        AssetRecordTemplate::with_no_asset_tracing(
            n,
            ASSET_TYPE_FRA,
            AssetRecordType::NonConfidentialAmount_NonConfidentialAssetType,
            *pk,
        )
    });

    for output in outputs {
        trans_builder
            .add_output(&output, None, None, None)
            .c(d!())?;
    }

    trans_builder
        .balance()
        .c(d!())?
        .create(TransferType::Standard)
        .c(d!())?
        .sign(owner_kp)
        .c(d!())?
        .transaction()
        .c(d!())
}
