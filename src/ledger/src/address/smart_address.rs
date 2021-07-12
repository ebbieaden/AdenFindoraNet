//! Define `SmartAddress` struct.
//!
//! It can compact more type of address.

use crate::utils::H160;
use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};
use hex::FromHex;
use ruc::*;
use serde::{Deserialize, Serialize};
use zei::serialization::ZeiFromToBytes;
use zei::xfr::sig::XfrPublicKey;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SmartAddress {
    Ethereum(H160),
    Xfr(XfrPublicKey),
    Other,
}

impl SmartAddress {
    /// Get Address's byte length, without address type code.
    pub const fn size(&self) -> usize {
        match self {
            SmartAddress::Ethereum(_) => 20,
            // ed25519 key length
            SmartAddress::Xfr(_) => 32,
            SmartAddress::Other => 0,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            SmartAddress::Xfr(addr) => wallet::public_key_to_base64(&addr),
            SmartAddress::Ethereum(addr) => addr.to_string(),
            _ => String::new()
        }
    }

    pub fn from_string(s: String) -> Result<Self> {
        if s.len() == 42 && &s[..2] == "0x" {
            // is Ethereum address
            let address_hex = &s[2..];
            let inner = <[u8; 20]>::from_hex(address_hex).c(d!())?;
            Ok(SmartAddress::Ethereum(H160(inner)))
        } else {
            let address = wallet::public_key_from_base64(&s)?;
            Ok(SmartAddress::Xfr(address))
        }
    }

    /// Convert SmartAddress to ethereum address.
    pub fn to_ethereum_address(&self) -> Result<String> {
        match self {
            SmartAddress::Ethereum(addr) => {
                Ok(String::from("0x") + &hex::encode(addr.0))
            }
            _ => Err(eg!("Must use ethereum address.")),
        }
    }

    /// Get SmartAddress's type code.
    ///
    /// Ethereum address is 1.
    /// If this address is unsupported, return 0xffff.
    const fn get_type_code(&self) -> u16 {
        match self {
            SmartAddress::Xfr(_) => 0,
            SmartAddress::Ethereum(_) => 1,
            SmartAddress::Other => 0xFFFF,
        }
    }

    /// Get SmartAddress's bytes represent.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes_length = self.size() + 2;
        let mut bytes = Vec::with_capacity(bytes_length);
        bytes
            .write_u16::<NetworkEndian>(self.get_type_code())
            .c(d!())?;
        match self {
            SmartAddress::Ethereum(addr) => bytes.extend_from_slice(&addr.0),
            SmartAddress::Xfr(addr) => bytes.extend_from_slice(addr.as_bytes()),
            SmartAddress::Other => (),
        };
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let addr_type = NetworkEndian::read_u16(bytes);
        let sa = match addr_type {
            0 => {
                // let ed25519_public = PublicKey::from_bytes(&bytes[2..]);
                SmartAddress::Xfr(XfrPublicKey::zei_from_bytes(&bytes[2..]).c(d!())?)
            }
            1 => {
                let mut inner = [0u8; 20];
                inner.copy_from_slice(&bytes[2..]);
                SmartAddress::Ethereum(H160(inner))
            }
            _ => SmartAddress::Other,
        };
        Ok(sa)
    }
}
