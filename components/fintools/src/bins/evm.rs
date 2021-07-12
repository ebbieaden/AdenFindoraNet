use clap::{crate_authors, crate_version, App, SubCommand};
use fintools::fns::get_keypair;
use fintools::fns::utils;
use ledger::address::SmartAddress;
use ledger::data_model::BLACK_HOLE_PUBKEY_STAKING;
use ruc::*;
use txn_builder::BuildsTransactions;

fn transfer_amount(amount: u64, address: String) -> Result<()> {
    let mut builder = utils::new_tx_builder()?;

    let kp = get_keypair()?;

    let transfer_op =
        utils::gen_transfer_op(&kp, vec![(&BLACK_HOLE_PUBKEY_STAKING, amount)])?;
    builder
        .add_operation(transfer_op)
        .add_operation_convert_account(
            &kp,
            SmartAddress::from_string(address).c(d!())?,
        )?;
    utils::send_tx(&builder.take_transaction())?;
    Ok(())
}

fn run() -> Result<()> {
    let transfer = SubCommand::with_name("transfer")
        .arg_from_usage(
            "-b --balance=<Balance> transfer balance from utxo fra to account fra",
        )
        .arg_from_usage("-a --address=<Address> transfer target address");

    let matchs = App::new("fe")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Findora evm compact operator tool")
        .subcommand(transfer)
        .get_matches();

    if let Some(m) = matchs.subcommand_matches("transfer") {
        let amount = m.value_of("balance").c(d!())?;
        let address = m.value_of("address").c(d!())?;
        transfer_amount(
            u64::from_str_radix(amount, 10).c(d!())?,
            String::from(address),
        )?
    }
    Ok(())
}

fn main() {
    run().unwrap()
}
