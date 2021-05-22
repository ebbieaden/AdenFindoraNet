use ruc::*;
use serde_derive::Deserialize;
use std::{convert::TryFrom, env, fs, path::Path};

pub struct ABCIConfig {
    pub abci_host: String,
    pub abci_port: u16,
    pub tendermint_host: String,
    pub tendermint_port: u16,
    pub submission_host: String,
    pub submission_port: u16,
    pub ledger_host: String,
    pub ledger_port: u16,
    pub query_host: String,
    pub query_port: u16,
}

#[derive(Deserialize)]
pub struct ABCIConfigStr {
    pub abci_host: String,
    pub abci_port: String,
    pub tendermint_host: String,
    pub tendermint_port: String,
    pub submission_host: String,
    pub submission_port: String,
    pub ledger_host: String,
    pub ledger_port: String,
}

impl TryFrom<ABCIConfigStr> for ABCIConfig {
    type Error = Box<dyn RucError>;
    fn try_from(cfg: ABCIConfigStr) -> Result<Self> {
        let query_host = cfg.ledger_host.clone();
        let ledger_port = cfg.ledger_port.parse::<u16>().c(d!())?;
        let query_port = ledger_port - 1;
        Ok(ABCIConfig {
            abci_host: cfg.abci_host,
            abci_port: cfg.abci_port.parse::<u16>().c(d!())?,
            tendermint_host: cfg.tendermint_host,
            tendermint_port: cfg.tendermint_port.parse::<u16>().c(d!())?,
            submission_host: cfg.submission_host,
            submission_port: cfg.submission_port.parse::<u16>().c(d!())?,
            ledger_host: cfg.ledger_host,
            ledger_port,
            query_host,
            query_port,
        })
    }
}

impl ABCIConfig {
    pub fn from_env() -> Result<ABCIConfig> {
        // tendermint -------> abci(host, port)
        let abci_host =
            std::env::var("ABCI_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
        let abci_port = std::env::var("ABCI_PORT")
            .unwrap_or_else(|_| "26658".to_owned())
            .parse::<u16>()
            .c(d!())?;

        // abci ----> tendermint(host, port)
        let tendermint_host =
            std::env::var("TENDERMINT_HOST").unwrap_or_else(|_| "localhost".to_owned());
        let tendermint_port = std::env::var("TENDERMINT_PORT")
            .unwrap_or_else(|_| "26657".to_owned())
            .parse::<u16>()
            .c(d!())?;

        // client ------> abci(host, port, for submission)
        let submission_host =
            std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
        let submission_port = std::env::var("SUBMISSION_PORT")
            .unwrap_or_else(|_| "8669".to_owned())
            .parse::<u16>()
            .c(d!())?;

        // client ------> abci(host, port, for ledger access)
        let ledger_host = submission_host.clone();
        let ledger_port = std::env::var("LEDGER_PORT")
            .unwrap_or_else(|_| "8668".to_owned())
            .parse::<u16>()
            .c(d!())?;

        let query_host = ledger_host.clone();
        let query_port = ledger_port - 1;

        Ok(ABCIConfig {
            abci_host,
            abci_port,
            tendermint_host,
            tendermint_port,
            submission_host,
            submission_port,
            ledger_host,
            ledger_port,
            query_host,
            query_port,
        })
    }

    pub fn from_file() -> Result<ABCIConfig> {
        env::args()
            .nth(1)
            .map(|p| Path::new(&p).join("abci").join("abci.toml"))
            .ok_or_else(|| eg!())
            .and_then(|p| fs::read_to_string(p).c(d!()))
            .and_then(|contents| toml::from_str::<ABCIConfigStr>(&contents).c(d!()))
            .and_then(|cfg_str| ABCIConfig::try_from(cfg_str).c(d!()))
    }
}
