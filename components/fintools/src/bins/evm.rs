use clap::{crate_authors, crate_version, App, SubCommand};
use fintools::fns::get_keypair;
use fintools::fns::utils;
use ledger::address::SmartAddress;
use ledger::data_model::BLACK_HOLE_PUBKEY;
use ruc::*;
use txn_builder::BuildsTransactions;

fn address_bind(eth: &str) -> Result<()> {
    let mut builder = utils::new_tx_builder()?;
    let bindded_eth_sa = SmartAddress::from_ethereum_address(eth)?;

    let kp = get_keypair()?;

    builder.add_operation_bind_address(&kp, bindded_eth_sa)?;
    utils::send_tx(&builder.take_transaction())?;

    Ok(())
}

fn address_unbind() -> Result<()> {
    let mut builder = utils::new_tx_builder()?;

    let kp = get_keypair()?;

    builder.add_operation_unbind_address(&kp)?;
    utils::send_tx(&builder.take_transaction())?;

    Ok(())
}

fn transfer_amount(amount: u64) -> Result<()> {
    let mut builder = utils::new_tx_builder()?;

    let kp = get_keypair()?;

    let transfer_op = utils::gen_transfer_op(&kp, vec![(&BLACK_HOLE_PUBKEY, amount)])?;
    builder
        .add_operation(transfer_op)
        .add_operation_convert_account(&kp)?;
    utils::send_tx(&builder.take_transaction())?;
    Ok(())
}

fn run() -> Result<()> {
    let address = SubCommand::with_name("address")
        .arg_from_usage("-b --bind 'bind fra address and eth address'")
        .arg_from_usage("-u --unbind 'unbind fra address or eth address'")
        // .arg_from_usage("-f --findora-addr=<Address> 'findora address'")
        .arg_from_usage("-e --ethereum-addr=<Address> 'ethereum address'");

    let transfer = SubCommand::with_name("transfer").arg_from_usage(
        "-a --amount=<Amount> transfer amount from utxo fra to account fra",
    );

    let matchs = App::new("fe")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Findora evm compact operator tool")
        .subcommand(address)
        .subcommand(transfer)
        .get_matches();

    if let Some(m) = matchs.subcommand_matches("address") {
        // let findora_address = m.value_of("findora-addr");
        let ethereum_address = m.value_of("ethereum-addr");
        if m.is_present("bind") {
            address_bind(ethereum_address.c(d!())?)?;
        } else if m.is_present("unbind") {
            address_unbind()?;
        }
    };

    if let Some(m) = matchs.subcommand_matches("transfer") {
        let amount = m.value_of("amount").c(d!())?;
        transfer_amount(u64::from_str_radix(amount, 10).c(d!())?)?
    }
    Ok(())
}

fn main() {
    run().unwrap()
}
