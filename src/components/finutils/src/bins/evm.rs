use baseapp::{Action, CheckFee, CheckNonce};
use clap::{crate_authors, crate_version, App, SubCommand};
use fintools::fns::get_keypair;
use fintools::fns::utils;
use fp_core::account::{MintOutput, TransferToUTXO};
use fp_core::crypto::{Address32, MultiSignature};
use fp_core::ecdsa::Pair;
use fp_core::transaction::UncheckedTransaction;
use ledger::address::SmartAddress;
use ledger::data_model::ASSET_TYPE_FRA;
use ledger::data_model::BLACK_HOLE_PUBKEY_STAKING;
use module_account::Action as AccountAction;
use ruc::*;
use std::str::FromStr;
use tendermint_rpc::Client;
use tokio::runtime::Runtime;
use txn_builder::BuildsTransactions;
use zei::xfr::sig::XfrKeyPair;

/// transfer utxo assets to account(ed25519 or ecdsa address) balance.
fn transfer_to_account(amount: u64, address: String) -> Result<()> {
    let mut builder = utils::new_tx_builder()?;

    let kp = get_keypair()?;
    let transfer_op = utils::gen_transfer_op(
        &kp,
        vec![(&BLACK_HOLE_PUBKEY_STAKING, amount)],
        false,
        false,
    )?;
    builder
        .add_operation(transfer_op)
        .add_operation_convert_account(
            &kp,
            SmartAddress::from_string(address).c(d!())?,
        )?;
    utils::send_tx(&builder.take_transaction())?;
    Ok(())
}

pub enum Keypair {
    ED25519(XfrKeyPair),
    ECDSA(Pair),
}

impl Keypair {
    pub fn sign(&self, data: &[u8]) -> MultiSignature {
        match self {
            Keypair::ECDSA(kp) => MultiSignature::from(kp.sign(data)),
            Keypair::ED25519(kp) => {
                MultiSignature::from(kp.get_sk_ref().sign(data, kp.get_pk_ref()))
            }
        }
    }
}

/// transfer to uxto assets from account(ed25519 or ecdsa address) balance.
fn transfer_from_account(
    amount: u64,
    address: String,
    eth_phrase: Option<&str>,
) -> Result<()> {
    let addr = wallet::public_key_from_base64(&address)?;
    let output = MintOutput {
        target: addr,
        amount,
        asset: ASSET_TYPE_FRA,
    };

    let (signer, kp) = if let Some(key_path) = eth_phrase {
        let kp = Pair::from_phrase(key_path, None)?.0;
        let signer = Address32::from(kp.public());
        (signer, Keypair::ECDSA(kp))
    } else {
        let kp = get_keypair()?;
        let signer = Address32::from(kp.get_pk());
        (signer, Keypair::ED25519(kp))
    };

    let tm_client = tendermint_rpc::HttpClient::new("http://127.0.0.1:26657").unwrap();
    let query_ret = Runtime::new()
        .unwrap()
        .block_on(tm_client.abci_query(
            Some(tendermint::abci::Path::from_str("module/account/nonce").unwrap()),
            serde_json::to_vec(&signer).unwrap(),
            None,
            false,
        ))
        .unwrap();
    let nonce = serde_json::from_slice::<u64>(query_ret.value.as_slice()).unwrap();

    let account_call = AccountAction::TransferToUTXO(TransferToUTXO {
        outputs: vec![output],
    });
    let action = Action::Account(account_call);
    let extra = (CheckNonce::new(nonce), CheckFee::new(None));
    let msg = serde_json::to_vec(&(action.clone(), extra.clone())).unwrap();

    let signature = kp.sign(msg.as_slice());

    let tx = UncheckedTransaction::new_signed(action, signer, signature, extra);

    let txn = serde_json::to_vec(&tx).unwrap();

    let resp = Runtime::new()
        .unwrap()
        .block_on(tm_client.broadcast_tx_commit(txn.into()))
        .c(d!())?;

    println!("tx_bytes: {:?}", resp);
    Ok(())
}

fn run() -> Result<()> {
    let deposit = SubCommand::with_name("deposit")
        .about("Transfer FRA from Findora account to an Ethereum account address")
        .arg_from_usage(
            "-a --address=<Address> 'Ethereum address to receive FRA, eg:0xd3Bf...'",
        )
        .arg_from_usage("-b --balance=<Balance> 'Deposit FRA amount'");

    let withdraw = SubCommand::with_name("withdraw")
        .about(
            "Transfer FRA from an ethereum account address \
         to the specified findora account address",
        )
        .arg_from_usage(
            "-a --address=<Address> 'Findora address to receive FRA, eg:fra1rkv...'",
        )
        .arg_from_usage("-b --balance=<Balance> 'Withdraw FRA amount'")
        .arg_from_usage(
            "-e --eth-key=[MNEMONIC] 'Ethereum account mnemonic phrase sign tx'",
        );

    let matchs = App::new("fe")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Findora evm compact operator tool")
        .subcommand(deposit)
        .subcommand(withdraw)
        .get_matches();

    if let Some(m) = matchs.subcommand_matches("deposit") {
        let amount = m.value_of("balance").c(d!())?;
        let address = m.value_of("address").c(d!())?;
        transfer_to_account(
            u64::from_str_radix(amount, 10).c(d!())?,
            String::from(address),
        )?
    }

    if let Some(m) = matchs.subcommand_matches("withdraw") {
        let amount = m.value_of("balance").c(d!())?;
        let address = m.value_of("address").c(d!())?;
        let eth_key = m.value_of("eth-key");
        transfer_from_account(
            u64::from_str_radix(amount, 10).c(d!())?,
            String::from(address),
            eth_key,
        )?
    }
    Ok(())
}

fn main() {
    run().unwrap()
}
