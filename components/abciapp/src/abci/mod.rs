#![deny(warnings)]

use lazy_static::lazy_static;
use ledger_api_service::RestfulApiService;
use ruc::*;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use submission_api::SubmissionApi;

mod config;
mod server;
pub mod staking;

use config::ABCIConfig;

lazy_static! {
    static ref LEDGER_DIR: Option<String> = env::var("LEDGER_DIR").ok();
}

pub fn run() -> Result<()> {
    let base_dir = if let Some(d) = LEDGER_DIR.as_ref() {
        fs::create_dir_all(d).c(d!())?;
        Some(Path::new(d))
    } else {
        None
    };

    let config = ruc::info!(ABCIConfig::from_file())
        .or_else(|_| ABCIConfig::from_env().c(d!()))?;

    let app = server::ABCISubmissionServer::new(
        base_dir,
        format!("{}:{}", config.tendermint_host, config.tendermint_port),
    )?;

    let submission_service_hdr = Arc::clone(&app.la);

    if env::var("ENABLE_LEDGER_SERVICE").is_ok() {
        let ledger_api_service_hdr =
            submission_service_hdr.read().borrowable_ledger_state();
        let address_binder = app.address_binder.clone();
        // let balance_store = app.balance_store.clone();
        let ledger_host = config.ledger_host.clone();
        let ledger_port = config.ledger_port;
        thread::spawn(move || {
            pnk!(RestfulApiService::create(
                ledger_api_service_hdr,
                address_binder,
                // balance_store,
                &ledger_host,
                ledger_port
            ));
        });
    }

    if env::var("ENABLE_QUERY_SERVICE").is_ok() {
        let query_service_hdr = submission_service_hdr.read().borrowable_ledger_state();
        pnk!(query_api::service::start_query_server(
            query_service_hdr,
            &config.query_host,
            config.query_port,
        ))
        .write()
        .update();
    }

    if env::var("ENABLE_ETH_API_SERVICE").is_ok() {
        eth_api_service::service::start();
    }

    let submission_host = config.submission_host.clone();
    let submission_port = config.submission_port;
    thread::spawn(move || {
        pnk!(SubmissionApi::create(
            submission_service_hdr,
            &submission_host,
            submission_port,
        ));
    });

    let addr_str = format!("{}:{}", config.abci_host, config.abci_port);
    let addr = addr_str.parse::<SocketAddr>().c(d!())?;

    // handle SIGINT signal
    ctrlc::set_handler(move || {
        std::process::exit(0);
    })
    .c(d!())?;

    abci::run(addr, app);
    Ok(())
}
