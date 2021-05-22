//!
//! Initial Config
//!

use super::{
    td_addr_to_bytes, BlockHeight, Power, Validator, ValidatorData, ValidatorKind,
    STAKING_VALIDATOR_MIN_POWER,
};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

const DEFAULT_POWER: Power = 10 * STAKING_VALIDATOR_MIN_POWER;

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
    commission_rate: Option<[u64; 2]>,
    memo: Option<String>,
}

impl TryFrom<ValidatorStr> for Validator {
    type Error = Box<dyn ruc::RucError>;
    fn try_from(v: ValidatorStr) -> Result<Validator> {
        Ok(Validator {
            td_pubkey: base64::decode(&v.td_pubkey).c(d!())?,
            td_addr: td_addr_to_bytes(&v.td_addr).c(d!())?,
            td_power: v.td_power.unwrap_or(DEFAULT_POWER),
            commission_rate: v.commission_rate.unwrap_or([1, 100]),
            id: wallet::public_key_from_base64(&v.id).c(d!())?,
            memo: v.memo,
            kind: ValidatorKind::Initor,
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
#[cfg(not(any(feature = "debug_env", feature = "abci_mock")))]
pub fn get_cfg_data() -> Result<InitialValidatorInfo> {
    const CFG: &str = r#"
        {
          "valiators": [
            {
              "id": "cF7onRo9F82AGD5c8d11EfuyYN77g6-_vsAuppfqxt8=",
              "td_addr": "FD8C65634A9D8899FA14200177AF19D24F6E1C37",
              "td_pubkey": "B5tUqZvHVAJw4xu5x5yFBm6uO9L28ZiqQ9SYNeTsx4A="
            },
            {
              "id": "7uSHa4M42_-qZaccutkidGRCP84Y-luNriQxBU3qRXI=",
              "td_addr": "0856654F7CD4BB0D6CC4409EF4892136C9D24692",
              "td_pubkey": "NkiURaWrhY6Ao2qgBX+ZbZIw2cqctUw5fevNnhqYrNQ="
            },
            {
              "id": "xLv_G-276O8cVnSVRAmo8KHjJmeBcV9_LdS7BBpWvCc=",
              "td_addr": "5C97EE9B91D90B332813078957E3A96B304791B4",
              "td_pubkey": "elrzoQ3aKkH+023XYDK2VAuzUBkHjcDhbzrfmZOX4M4="
            },
            {
              "id": "RcUUC2x-yxIZ7O0antSgN-Yl6ET1MmN-ZX9xjt9sy8I=",
              "td_addr": "000E33AB7471186F3B1DE9FC08BB9C480F453590",
              "td_pubkey": "H6wmuTEul46sCvwDUXCtYRxtW6xiVAwwa9pezrP2o80="
            },
            {
              "id": "2mevsiKm4-wWImyUOivNTVyecRjXCO2x5NqkFu4cxlA=",
              "td_addr": "EA70EB6087E3D606730C4E9062CC24A5BD7D2B37",
              "td_pubkey": "I9GrRvKlzS48VyxpDJr+O3574ibsFjIZpQzU7t74b8o="
            },
            {
              "id": "0wobjTwVCzH68WCEv4vlzu4dWTK2O0k3yxdt0iOX5Bc=",
              "td_addr": "E5705FED0049EDA431D37B37947A136F22F8F054",
              "td_pubkey": "+1dMQrGaVrjWLRbTxFRtoBLH2s+NYWvjiLL0jlNHi/w="
            },
            {
              "id": "W4b1crCUKbDCyGMK2M9AXqGUmC4lAxRMeswb1gAPoIo=",
              "td_addr": "9ED0D8D661C99A58F78F80816968E61AAE8DC649",
              "td_pubkey": "6LGsL/tD5LLZW4tXQYVqJRIg6Vz8r1OOCrrG6p53RIo="
            },
            {
              "id": "EtQDh6fS9Adj10Pro9VocnPvcCuPua81UHaLELwilfY=",
              "td_addr": "9AB077E00C8B731AE1F82DEC5E45CB3D1E9BBB12",
              "td_pubkey": "p1W9CxtMCH1RcH83zOWHENRWjJlPDdURAlPozM2Arw0="
            },
            {
              "id": "MOor1DGWz87B-l9ib0ntoxxINcyRR_dr-BDHGWK3dnA=",
              "td_addr": "8CB713C8EA32223FCAC66B966FCFA9BAEE257946",
              "td_pubkey": "BCCnRcAWYfM8wQ8NXc5PPlxho2a9jSXAeT0sVnMmkjg="
            },
            {
              "id": "7PZGYAG3Z1OrlOjCymmo8yr0z9QeoYxL0CU634kiMI8=",
              "td_addr": "EAC5792572EB726AA0DBA9A7AFA9757F8063C6C9",
              "td_pubkey": "H25b+bRch0oH6sdRyr72gmP/+NpHV7yaZEk3QtDhSUA="
            },
            {
              "id": "8zSgShT-I4XpsXUNCdCa0Z3RBp6lKuaEqZE_XGKFo1A=",
              "td_addr": "A50D65F2F63F65D845A7C5CBB989FF94D6688F38",
              "td_pubkey": "hgFW7c97SrmmTu5Neq8iaGKOIETJQ+2Yy43OAScRasM="
            },
            {
              "id": "AGLlaDJHS5zr1D-M_WAe785IrJvMNG3xTCpa1QQOJdE=",
              "td_addr": "A8DFD116BA9664F38958C721688FA73E6320755B",
              "td_pubkey": "TrKAw22tqstm8mLSihU8Zcaq+ujAM+SeAQldCeyNfeA="
            },
            {
              "id": "PevSEiTLzWfvkTEES6BBWSuKF43FbHBnOA684M6_Nz4=",
              "td_addr": "A07875BBD4E062BAB2C162E180237FC3B30C4ABC",
              "td_pubkey": "tTLSfzxCspD7o2xOg8FpU8LP0Z4/8g0hYfkcrHyB054="
            },
            {
              "id": "ezH7VOnEYf3QB2VG8b-GsqsqQhGfwC0TO5C-TD6zqf8=",
              "td_addr": "39F0C5E451394FAAE7213FD914EFBA8F963CCB90",
              "td_pubkey": "WA6+6DX7ezKEjShNsZzvBBL5uhqubMDt7D54PSbJURQ="
            },
            {
              "id": "k3MCjemGk_WfQHqE2X9pBkxXK8rV8B7bf72XMi0mAXg=",
              "td_addr": "EE2F73BAA1605C998BB106E5A38DBD79B5209F1D",
              "td_pubkey": "ZQH2pfH00RuClYUU5J4pnb+zCUK0iplcYmewZ9H9QU8="
            },
            {
              "id": "cEbZNU2PTPnKgEu4Auq-N31I6kl5N64guQ7h4iZbHKo=",
              "td_addr": "09EF1DB6B67D1CBF7EBA6BD9B204611848993DF7",
              "td_pubkey": "vuV4K1sAS0F255kc7FgZxK6/rmP/nMJQ3qYC3zqMOC8="
            },
            {
              "id": "W-hRhXdwBOnUJLZ3U3Kp-a8eEY8KFa4qwFrQxqyO6uU=",
              "td_addr": "AD2C69A9432E8F6634E1ADC3D6CA69EA9E1F4114",
              "td_pubkey": "UAvGQOeMsEQNR89cDK7fGELhCvMLdC8bo1BG/77qwd8="
            },
            {
              "id": "p8p0SBhweFmVtxelXjwDuhksZngA5bMWZOroS56rd9E=",
              "td_addr": "510082967DFA7DEBA11267B26A6318D07A457B48",
              "td_pubkey": "h/DkJo3TsoxZR9yINi3I0AOTZAYOF1gTJw9iQwcp0qA="
            },
            {
              "id": "NHgS1jnli7zKcYQuMFq0xA0c-pzpYVTgE1_00cbaEgo=",
              "td_addr": "60689516C566F27E03794329C431D0084299480A",
              "td_pubkey": "k/jv9U6MiMNUjh9ZigHiQSLgereJhxFdgrTHEmxmnes="
            },
            {
              "id": "A1vNVGGWiP6hdpUE1JJXrpTVpUuUDOV71YvanzWA7LA=",
              "td_addr": "5C71532CEEFC43EE3857905AB94FDA505BFC06F3",
              "td_pubkey": "8bQ1CgoaWCzpemN3oZGAsf+lmdmuysNozMyAvpE2lBY="
            }
          ]
        }
    "#;

    serde_json::from_str(CFG).c(d!())
}

#[allow(missing_docs)]
#[cfg(any(feature = "debug_env", feature = "abci_mock"))]
pub fn get_cfg_data() -> Result<InitialValidatorInfo> {
    get_cfg_path()
        .c(d!())
        .and_then(|f| std::fs::read(f).c(d!()))
        .and_then(|v| serde_json::from_slice::<InitialValidatorInfo>(&v).c(d!()))
}

/// used in `cfg_generator` binary
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
