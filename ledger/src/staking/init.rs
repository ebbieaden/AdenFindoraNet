use super::{BlockHeight, Validator, ValidatorData};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fs};

// Generate config during compiling time.
#[derive(Serialize, Deserialize)]
struct InitialValidatorInfo {
    height: BlockHeight,
    valiators: Vec<ValidatorStr>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
struct ValidatorStr {
    // Tendermint PubKey, in base64 format
    td_pubkey: String,
    td_power: i64,
    // `XfrPublicKey` in base64 format
    id: String,
    memo: Option<String>,
}

impl TryFrom<ValidatorStr> for Validator {
    type Error = Box<dyn ruc::RucError>;
    fn try_from(v: ValidatorStr) -> Result<Validator> {
        Ok(Validator {
            td_pubkey: base64::decode(&v.td_pubkey).c(d!())?,
            td_power: v.td_power,
            id: wallet::public_key_from_base64(&v.id).c(d!())?,
            memo: v.memo,
        })
    }
}

// **Return:**
// - the initial height when do upgrading
// - the initial validator-set informations
pub(super) fn get_inital_validators() -> Result<ValidatorData> {
    option_env!("STAKING_INITIAL_VALIDATOR_INFO_CONFIG")
        .c(d!())
        .and_then(|f| fs::read(f).c(d!()))
        .and_then(|v| serde_json::from_slice::<InitialValidatorInfo>(&v).c(d!()))
        .and_then(|i| {
            let h = i.height;
            i.valiators
                .into_iter()
                .map(|v| Validator::try_from(v).c(d!()))
                .collect::<Result<Vec<_>>>()
                .c(d!())
                .and_then(|v| ValidatorData::new(h, v).c(d!()))
        })
}
