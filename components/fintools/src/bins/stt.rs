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
use fintools::fns::utils as fns;
use lazy_static::lazy_static;
use ledger::{
    data_model::{Transaction, BLACK_HOLE_PUBKEY_STAKING},
    staking::{
        check_delegation_amount, gen_random_keypair, td_addr_to_bytes, BLOCK_INTERVAL,
        FRA, FRA_TOTAL_AMOUNT,
    },
    store::fra_gen_initial_tx,
};
use ruc::*;
use serde::Serialize;
use std::{collections::BTreeMap, env};
use txn_builder::BuildsTransactions;
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey};

lazy_static! {
    static ref USER_LIST: BTreeMap<Name, User> = gen_user_list();
    static ref VALIDATOR_LIST: BTreeMap<Name, Validator> = gen_valiator_list();
    static ref ROOT_KP: XfrKeyPair =
        pnk!(wallet::restore_keypair_from_mnemonic_default(ROOT_MNEMONIC));
}

const ROOT_MNEMONIC: &str = "bright poem guard trade airport artist soon mountain shoe satisfy fox adapt garden decline uncover when pilot person flat bench connect coach planet hidden";

type Name = String;
type NameRef<'a> = &'a str;

macro_rules! sleep_n_block {
    ($n_block: expr) => {
        sleep_ms!($n_block * BLOCK_INTERVAL * 1000);
    };
}

fn main() {
    pnk!(run());
}

fn run() -> Result<()> {
    let subcmd_init = SubCommand::with_name("init");
    let subcmd_issue = SubCommand::with_name("issue");
    let subcmd_delegate = SubCommand::with_name("delegate")
        .arg_from_usage("-u, --user=[User] 'user name of delegator'")
        .arg_from_usage("-n, --amount=[Amount] 'how much FRA units to delegate'")
        .arg_from_usage("-v, --validator=[Validator] 'which validator to delegate to'");
    let subcmd_undelegate = SubCommand::with_name("undelegate")
        .arg_from_usage("-u, --user=[User] 'user name of the delegator'")
        .arg_from_usage("-n, --amount=[Amount] 'how much FRA to undelegate, needed for partial undelegation'")
        .arg_from_usage("-v, --validator=[Validator] 'which validator to undelegate from, needed for partial undelegation'");
    let subcmd_claim = SubCommand::with_name("claim")
        .arg_from_usage("-u, --user=[User] 'user name of delegator'")
        .arg_from_usage("-n, --amount=[Amount] 'how much FRA to claim'");
    let subcmd_transfer = SubCommand::with_name("transfer")
        .arg_from_usage("-f, --from-user=[User] 'transfer sender'")
        .arg_from_usage("-t, --to-user=[User] 'transfer receiver'")
        .arg_from_usage("-n, --amount=[Amount] 'how much FRA to transfer'");
    let subcmd_show = SubCommand::with_name("show")
        .arg_from_usage("-r, --root-mnemonic 'show the pre-defined root mnemonic'")
        .arg_from_usage("-U, --user-list 'show the pre-defined user list'")
        .arg_from_usage("-v, --validator-list 'show the pre-defined validator list'")
        .arg_from_usage("-u, --user=[User] 'user name of delegator'");

    let matches = App::new("stt")
        .version(crate_version!())
        .author(crate_authors!())
        .about("A manual test tool for the staking function.")
        .subcommand(subcmd_init)
        .subcommand(subcmd_issue)
        .subcommand(subcmd_delegate)
        .subcommand(subcmd_undelegate)
        .subcommand(subcmd_claim)
        .subcommand(subcmd_transfer)
        .subcommand(subcmd_show)
        .get_matches();

    if matches.is_present("init") {
        init::init().c(d!())?;
    } else if matches.is_present("issue") {
        issue::issue().c(d!())?;
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
                .and_then(|tx| fns::send_tx(&tx).c(d!()))?;
        }
    } else if let Some(m) = matches.subcommand_matches("undelegate") {
        let user = m.value_of("user");
        let amount = m.value_of("amount");
        let validator = m.value_of("validator");

        if user.is_none()
            || user.unwrap().trim().is_empty()
            || matches!((amount, validator), (Some(_), None) | (None, Some(_)))
        {
            println!("{}", m.usage());
        } else {
            let amount = amount.and_then(|am| am.parse::<u64>().ok());
            undelegate::gen_tx(user.unwrap(), amount, validator)
                .c(d!())
                .and_then(|tx| fns::send_tx(&tx).c(d!()))?;
        }
    } else if let Some(m) = matches.subcommand_matches("claim") {
        let user = m.value_of("user");

        if user.is_none() {
            println!("{}", m.usage());
        } else {
            let amount = if let Some(am) = m.value_of("amount") {
                Some(am.parse::<u64>().c(d!())?)
            } else {
                None
            };
            claim::gen_tx(user.unwrap(), amount)
                .c(d!())
                .and_then(|tx| fns::send_tx(&tx).c(d!()))?;
        }
    } else if let Some(m) = matches.subcommand_matches("transfer") {
        let from = m.value_of("from-user");
        let to = m.value_of("to-user");
        let amount = m.value_of("amount");

        match (from, to, amount) {
            (Some(sender), Some(receiver), Some(am)) => {
                let am = am.parse::<u64>().c(d!())?;
                let owner_kp = search_kp(sender).c(d!())?;
                let target_pk = search_kp(receiver)
                    .c(d!())
                    .map(|kp| kp.get_pk())
                    .or_else(|e| wallet::public_key_from_base64(receiver).c(d!(e)))?;
                fns::transfer(owner_kp, &target_pk, am).c(d!())?;
            }
            _ => {
                println!("{}", m.usage());
            }
        }
    } else if let Some(m) = matches.subcommand_matches("show") {
        let rm = m.is_present("root-mnemonic");
        let ul = m.is_present("user-list");
        let vl = m.is_present("validator-list");
        let u = m.value_of("user");

        if rm || ul || vl || u.is_some() {
            print_info(rm, ul, vl, u).c(d!())?;
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

        println!(">>> define and issue FRA...");
        fns::send_tx(&fra_gen_initial_tx(&root_kp)).c(d!())?;

        println!(">>> wait 4 blocks...");
        sleep_n_block!(4);

        println!(">>> set initial validator set...");
        fns::set_initial_validators(&root_kp).c(d!())?;

        println!(">>> wait 4 blocks...");
        sleep_n_block!(4);

        let target_list = USER_LIST
            .values()
            .map(|u| &u.pubkey)
            .chain(VALIDATOR_LIST.values().map(|v| &v.pubkey))
            .map(|pk| (pk, FRA_TOTAL_AMOUNT / 10000))
            .collect::<Vec<_>>();

        println!(">>> transfer FRAs to validators...");
        fns::transfer_batch(&root_kp, target_list).c(d!())?;

        println!(">>> wait 6 blocks ...");
        sleep_n_block!(6);

        println!(">>> propose self-delegations...");
        for v in VALIDATOR_LIST.values() {
            delegate::gen_tx(&v.name, FRA, &v.name)
                .c(d!())
                .and_then(|tx| fns::send_tx(&tx).c(d!()))?;
        }

        println!(">>> DONE !");
        Ok(())
    }
}

mod issue {
    use super::*;
    use ledger::{
        data_model::{
            AssetTypeCode, IssueAsset, IssueAssetBody, IssuerKeyPair, Operation,
            TxOutput, ASSET_TYPE_FRA,
        },
        staking::FRA_TOTAL_AMOUNT,
    };
    use rand_chacha::rand_core::SeedableRng;
    use rand_chacha::ChaChaRng;
    use zei::setup::PublicParams;
    use zei::xfr::{
        asset_record::{build_blind_asset_record, AssetRecordType},
        structs::AssetRecordTemplate,
    };

    pub fn issue() -> Result<()> {
        gen_issue_tx()
            .c(d!())
            .and_then(|tx| fns::send_tx(&tx).c(d!()))
    }

    fn gen_issue_tx() -> Result<Transaction> {
        let root_kp =
            wallet::restore_keypair_from_mnemonic_default(ROOT_MNEMONIC).c(d!())?;

        let mut builder = fns::new_tx_builder().c(d!())?;

        let template = AssetRecordTemplate::with_no_asset_tracing(
            FRA_TOTAL_AMOUNT / 2,
            ASSET_TYPE_FRA,
            AssetRecordType::NonConfidentialAmount_NonConfidentialAssetType,
            root_kp.get_pk(),
        );
        let params = PublicParams::default();
        let outputs = (0..2)
            .map(|_| {
                let (ba, _, _) = build_blind_asset_record(
                    &mut ChaChaRng::from_entropy(),
                    &params.pc_gens,
                    &template,
                    vec![],
                );
                (
                    TxOutput {
                        id: None,
                        record: ba,
                        lien: None,
                    },
                    None,
                )
            })
            .collect::<Vec<_>>();
        let aib = IssueAssetBody::new(
            &AssetTypeCode {
                val: ASSET_TYPE_FRA,
            },
            builder.get_seq_id(),
            &outputs,
        )
        .c(d!())?;
        let asset_issuance_operation =
            IssueAsset::new(aib, &IssuerKeyPair { keypair: &root_kp }).c(d!())?;

        builder.add_operation(Operation::IssueAsset(asset_issuance_operation));
        Ok(builder.take_transaction())
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

        let mut builder = fns::new_tx_builder().c(d!())?;

        fns::gen_transfer_op(owner_kp, vec![(&BLACK_HOLE_PUBKEY_STAKING, amount)])
            .c(d!())
            .map(|principal_op| {
                builder.add_operation(principal_op);
                builder.add_operation_delegation(owner_kp, validator.to_owned());
            })?;

        Ok(builder.take_transaction())
    }
}

mod undelegate {
    use super::*;
    use ledger::staking::PartialUnDelegation;

    pub fn gen_tx(
        user: NameRef,
        amount: Option<u64>,
        validator: Option<NameRef>,
    ) -> Result<Transaction> {
        let owner_kp = &USER_LIST.get(user.trim()).c(d!())?.keypair;
        let validator = validator
            .and_then(|v| VALIDATOR_LIST.get(v))
            .map(|x| pnk!(td_addr_to_bytes(&x.td_addr)));

        let mut builder = fns::new_tx_builder().c(d!())?;

        fns::gen_fee_op(owner_kp).c(d!()).map(|op| {
            builder.add_operation(op);
            if let Some(amount) = amount {
                // partial undelegation
                builder.add_operation_undelegation(
                    owner_kp,
                    Some(PartialUnDelegation::new(
                        amount,
                        gen_random_keypair().get_pk(),
                        validator.unwrap(),
                    )),
                );
            } else {
                builder.add_operation_undelegation(owner_kp, None);
            }
        })?;

        Ok(builder.take_transaction())
    }
}

mod claim {
    use super::*;

    pub fn gen_tx(user: NameRef, amount: Option<u64>) -> Result<Transaction> {
        let owner_kp = search_kp(user).c(d!())?;

        let mut builder = fns::new_tx_builder().c(d!())?;

        fns::gen_fee_op(owner_kp).c(d!()).map(|op| {
            builder.add_operation(op);
            builder.add_operation_claim(owner_kp, amount);
        })?;

        Ok(builder.take_transaction())
    }
}

fn print_info(
    show_root_mnemonic: bool,
    show_user_list: bool,
    show_validator_list: bool,
    user: Option<NameRef>,
) -> Result<()> {
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

fn get_delegation_info(user: NameRef) -> Result<String> {
    let pk = USER_LIST
        .get(user)
        .map(|u| &u.pubkey)
        .or_else(|| VALIDATOR_LIST.get(user).map(|v| &v.pubkey))
        .c(d!())?;

    fns::get_delegation_info(pk)
        .c(d!())
        .and_then(|di| serde_json::to_string_pretty(&di).c(d!()))
}

fn get_balance(user: NameRef) -> Result<u64> {
    let kp = search_kp(user).c(d!())?;
    fns::get_balance(&kp).c(d!())
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

    // locale env
    #[cfg(feature = "debug_env")]
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

    // online env
    #[cfg(not(feature = "debug_env"))]
    const TD_ADDR_LIST: [&str; NUM] = [
        "FD8C65634A9D8899FA14200177AF19D24F6E1C37",
        "0856654F7CD4BB0D6CC4409EF4892136C9D24692",
        "5C97EE9B91D90B332813078957E3A96B304791B4",
        "000E33AB7471186F3B1DE9FC08BB9C480F453590",
        "EA70EB6087E3D606730C4E9062CC24A5BD7D2B37",
        "E5705FED0049EDA431D37B37947A136F22F8F054",
        "9ED0D8D661C99A58F78F80816968E61AAE8DC649",
        "9AB077E00C8B731AE1F82DEC5E45CB3D1E9BBB12",
        "8CB713C8EA32223FCAC66B966FCFA9BAEE257946",
        "EAC5792572EB726AA0DBA9A7AFA9757F8063C6C9",
        "A50D65F2F63F65D845A7C5CBB989FF94D6688F38",
        "A8DFD116BA9664F38958C721688FA73E6320755B",
        "A07875BBD4E062BAB2C162E180237FC3B30C4ABC",
        "39F0C5E451394FAAE7213FD914EFBA8F963CCB90",
        "EE2F73BAA1605C998BB106E5A38DBD79B5209F1D",
        "09EF1DB6B67D1CBF7EBA6BD9B204611848993DF7",
        "AD2C69A9432E8F6634E1ADC3D6CA69EA9E1F4114",
        "510082967DFA7DEBA11267B26A6318D07A457B48",
        "60689516C566F27E03794329C431D0084299480A",
        "5C71532CEEFC43EE3857905AB94FDA505BFC06F3",
    ];

    #[cfg(feature = "debug_env")]
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

    #[cfg(not(feature = "debug_env"))]
    const MNEMONIC_LIST: [&str; NUM] = [
        "arrest wrist tumble fall agree tunnel modify soldier arrange step stadium taste special lawn illness village abstract wheel opera fit define device burden relief",
        "detail frog laundry clay border urge one olympic liberty buddy capital zone catalog lunar special double design consider banana round achieve desert ride cup",
        "silk garlic area tell sudden bird dove shield question powder visa limit stool column soccer unfair tobacco online right front valid uniform ski private",
        "humor shop duck festival pottery rescue proof roof galaxy become arrow seat safe monster labor treat adapt comfort almost cousin yellow install vote mother",
        "town egg feature credit drum rescue evil asthma defy artist where amazing ankle syrup awake drop magnet mandate crucial phone cinnamon fabric window address",
        "negative health science tiger palm dutch memory approve icon fresh moral stumble enable trash cushion cherry viable hazard pull reward shift mansion blur nothing",
        "harsh list identify fortune kick feel naive spy universe jacket glow comic nice hammer fire truth vessel bind private round quiz elite broken roof",
        "stumble tank law captain girl bitter spring tumble reveal execute tent junk artist pudding sponsor city ginger denial copy addict canvas explain favorite indicate",
        "mixed snack ribbon question addict situate suit letter left beach guitar tobacco tank strike can zebra sniff distance mirror gospel weekend gown candy moon",
        "iron vapor hello easily level concert second undo elbow question anxiety stove merry young rate sound tackle cash trim eternal dog curtain charge random",
        "trigger usage enforce sunset bless sorry hard return disorder window muffin assume only sight census aisle hamster brush accident science pear addict saddle pole",
        "income sign bargain unit ready scale inner nice flash coach oval about price assume add seminar claim open rescue body flame meat supply expire",
        "alert orient second industry power shove finger culture evil scout view cream win poem power protect journey build tide work picture suggest velvet inner",
        "concert corn drama emerge amateur blur boss travel burden reflect time old whisper lens roast spider kiwi derive stick mimic girl burger joy vault",
        "practice nut satisfy embark rug desk cloud donkey gloom simple disorder have one plate spawn result chase pond canoe main essence degree often industry",
        "dragon alien deal appear cost arm shop drama heart satoshi bring lion fruit cross cradle book now verb way arrange observe wide rubber combine",
        "buffalo twice awake erode marriage oyster reunion foot favorite boost happy boring eight flower return core planet adjust funny asset luxury bean double trim",
        "town birth juice near group imitate dismiss alpha ginger october antenna rookie next suspect mountain pilot prosper fashion shell submit swap atom catalog misery",
        "ugly talent maximum relax frame subject chronic member fade seven range crop goat happy trumpet element among tube dune ten sheriff mule mouse glad",
        "health summer sausage steel salon ridge pudding stone museum obey size panther crush pizza between start deny sheriff naive frost mechanic thing garden hollow",
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

fn search_kp(user: NameRef) -> Option<&'static XfrKeyPair> {
    if "root" == user {
        return Some(&ROOT_KP);
    }

    USER_LIST
        .get(user)
        .map(|u| &u.keypair)
        .or_else(|| VALIDATOR_LIST.get(user).map(|v| &v.keypair))
}
