//!
//! Initial Config
//!

use super::{BlockHeight, Power, Validator, ValidatorData, FRA};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fs};

const DEFAULT_POWER: Power = 32_0000 * FRA;

/// Generate config during compiling time.
#[derive(Serialize, Deserialize)]
pub struct InitialValidatorInfo {
    height: Option<BlockHeight>,
    /// predefined validators
    pub valiators: Vec<ValidatorStr>,
}

/// Used for parsing config from disk.
#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct ValidatorStr {
    /// `XfrPublicKey` in base64 format
    pub id: String,
    // Tendermint Addr, in hex format
    td_addr: String,
    // Tendermint PubKey, in base64 format
    td_pubkey: String,
    td_power: Option<Power>,
    memo: Option<String>,
}

impl TryFrom<ValidatorStr> for Validator {
    type Error = Box<dyn ruc::RucError>;
    fn try_from(v: ValidatorStr) -> Result<Validator> {
        Ok(Validator {
            td_pubkey: base64::decode(&v.td_pubkey).c(d!())?,
            td_addr: hex::decode(&v.td_addr).c(d!())?,
            td_power: v.td_power.unwrap_or(DEFAULT_POWER),
            id: wallet::public_key_from_base64(&v.id).c(d!())?,
            memo: v.memo,
        })
    }
}

// **Return:**
// - the initial height when do upgrading
// - the initial validator-set informations
pub(super) fn get_inital_validators() -> Result<ValidatorData> {
    get_cfg_data().c(d!()).and_then(|i| {
        let h = i.height.unwrap_or(1);
        i.valiators
            .into_iter()
            .map(|v| Validator::try_from(v).c(d!()))
            .collect::<Result<Vec<_>>>()
            .c(d!())
            .and_then(|v| ValidatorData::new(h, v).c(d!()))
    })
}

#[allow(missing_docs)]
pub fn get_cfg_data() -> Result<InitialValidatorInfo> {
    get_cfg_path()
        .c(d!())
        .and_then(|f| fs::read(f).c(d!()))
        .and_then(|v| serde_json::from_slice::<InitialValidatorInfo>(&v).c(d!()))
}

#[allow(missing_docs)]
#[cfg(not(any(feature = "debug_env", feature = "abci_mock")))]
pub fn get_cfg_path() -> Option<&'static str> {
    option_env!("STAKING_INITIAL_VALIDATOR_CONFIG")
}

#[allow(missing_docs)]
#[cfg(feature = "debug_env")]
pub fn get_cfg_path() -> Option<&'static str> {
    option_env!("STAKING_INITIAL_VALIDATOR_CONFIG_DEBUG_ENV")
}

#[allow(missing_docs)]
#[cfg(feature = "abci_mock")]
pub fn get_cfg_path() -> Option<&'static str> {
    option_env!("STAKING_INITIAL_VALIDATOR_CONFIG_ABCI_MOCK")
}

#[cfg(test)]
#[cfg(feature = "abci_mock")]
mod test {
    use super::*;
    use crate::staking::td_pubkey_to_td_addr;

    #[test]
    fn staking_tendermint_addr_conversion() {
        let data = pnk!(get_cfg_data()).valiators;
        data.into_iter().for_each(|v| {
            let pk = pnk!(base64::decode(&v.td_pubkey));
            assert_eq!(v.td_addr, td_pubkey_to_td_addr(&pk));
        });
    }
}
