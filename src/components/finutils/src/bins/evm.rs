use clap::{crate_authors, crate_version, App, SubCommand};
use fintools::fns::get_keypair;
use fintools::fns::utils;
use ledger::address::SmartAddress;
use ruc::*;
use txn_builder::BuildsTransactions;

fn address_bind(eth: &str) -> Result<()> {
    let mut builder = utils::new_tx_builder().c(d!())?;
    let bindded_eth_sa = SmartAddress::from_ethereum_address(eth)?;

    let kp = get_keypair()?;

    builder.add_operation_bind_address(&kp, bindded_eth_sa)?;
    utils::send_tx(&builder.take_transaction()).c(d!())?;

    Ok(())
}

fn address_unbind() -> Result<()> {
    let mut builder = utils::new_tx_builder().c(d!())?;

    let kp = get_keypair()?;

    builder.add_operation_unbind_address(&kp)?;
    utils::send_tx(&builder.take_transaction()).c(d!())?;

    Ok(())
}

fn run() -> Result<()> {
    let address = SubCommand::with_name("address")
        .arg_from_usage("-b --bind 'bind fra address and eth address'")
        .arg_from_usage("-u --unbind 'unbind fra address or eth address'")
        // .arg_from_usage("-f --findora-addr=<Address> 'findora address'")
        .arg_from_usage("-e --ethereum-addr=<Address> 'ethereum address'");

    let matchs = App::new("fe")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Findora evm compact operator tool")
        .subcommand(address)
        .get_matches();

    if let Some(m) = matchs.subcommand_matches("address") {
        // let findora_address = m.value_of("findora-addr");
        let ethereum_address = m.value_of("ethereum-addr");
        if m.is_present("bind") {
            address_bind(ethereum_address.c(d!())?).c(d!())?;
        } else if m.is_present("unbind") {
            address_unbind().c(d!())?;
        }
    };
    Ok(())
}

fn main() {
    run().unwrap()
}
