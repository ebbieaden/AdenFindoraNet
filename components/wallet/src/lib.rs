//!
//! Generate mnemonic and restore keypair from it.
//!
//! Separating mnemonic to a standalone library is needed by tests.
//!

use bip39::{Language, Mnemonic, MnemonicType, Seed};
use ed25519_dalek_bip32::{DerivationPath, ExtendedSecretKey};
use ruc::{err::*, *};
use std::result::Result as Res;
use zei::{
    serialization::ZeiFromToBytes,
    xfr::sig::{XfrKeyPair, XfrSecretKey},
};

/// Randomly generate a 12words-length mnemonic.
pub fn generate_mnemonic_default() -> String {
    Mnemonic::new(MnemonicType::Words12, Language::English).into_phrase()
}

/// Generate mnemonic with custom length and language.
/// - @param `wordslen`: acceptable value are one of [ 12, 15, 18, 21, 24 ]
/// - @param `lang`: acceptable value are one of [ "en", "zh", "zh_traditional", "fr", "it", "ko", "sp", "jp" ]
pub fn generate_mnemonic_custom(wordslen: u8, lang: &str) -> Res<String, String> {
    let w = match wordslen {
        12 => MnemonicType::Words12,
        15 => MnemonicType::Words15,
        18 => MnemonicType::Words18,
        21 => MnemonicType::Words21,
        24 => MnemonicType::Words24,
        _ => {
            return Err(
                "Invalid words length, only 12/15/18/21/24 can be accepted.".to_owned()
            );
        }
    };

    let l = check_lang(lang).map_err(|e| genlog(e))?;

    Ok(Mnemonic::new(w, l).into_phrase())
}

// do the real restore operation.
macro_rules! restore_keypair_from_mnemonic {
    ($phrase: expr, $l: expr, $p: expr, $bip: tt) => {
        check_lang($l)
            .c(d!())
            .and_then(|l| Mnemonic::from_phrase($phrase, l).map_err(|e| eg!(1, e)))
            .map(|m| Seed::new(&m, ""))
            .and_then(|seed| {
                DerivationPath::$bip($p.coin, $p.account, $p.change, $p.address)
                    .map_err(|e| eg!(2, e))
                    .map(|dp| (seed, dp))
            })
            .and_then(|(seed, dp)| {
                ExtendedSecretKey::from_seed(seed.as_bytes())
                    .map_err(|e| eg!(3, e))?
                    .derive(&dp)
                    .map_err(|e| eg!(4, e))
            })
            .and_then(|kp| {
                XfrSecretKey::zei_from_bytes(&kp.secret_key.to_bytes()[..])
                    .map_err(|e| eg!(5, e))
            })
            .map(|sk| sk.into_keypair())
    };
}

/// Use this struct to express a Bip44/Bip49 path.
pub struct BipPath {
    coin: u32,
    account: u32,
    change: u32,
    address: u32,
}

impl BipPath {
    pub fn new(coin: u32, account: u32, change: u32, address: u32) -> Self {
        BipPath {
            coin,
            account,
            change,
            address,
        }
    }
}

/// Restore the XfrKeyPair from a mnemonic with a default bip44-path,
/// that is "m/44'/917'/0'/0/0" ("m/44'/coin'/account'/change/address").
pub fn restore_keypair_from_mnemonic_default(phrase: &str) -> Res<XfrKeyPair, String> {
    const FRA: u32 = 917;
    restore_keypair_from_mnemonic!(phrase, "en", BipPath::new(FRA, 0, 0, 0), bip44)
        .map_err(|e| genlog(e))
}

/// Restore the XfrKeyPair from a mnemonic with custom params,
/// in bip44 form.
pub fn restore_keypair_from_mnemonic_bip44(
    phrase: &str,
    lang: &str,
    path: &BipPath,
) -> Res<XfrKeyPair, String> {
    restore_keypair_from_mnemonic_bip44_inner(phrase, lang, path).map_err(|e| genlog(e))
}

#[inline(always)]
fn restore_keypair_from_mnemonic_bip44_inner(
    phrase: &str,
    lang: &str,
    path: &BipPath,
) -> Result<XfrKeyPair> {
    restore_keypair_from_mnemonic!(phrase, lang, path, bip44).c(d!())
}

/// Restore the XfrKeyPair from a mnemonic with custom params,
/// in bip49 form.
pub fn restore_keypair_from_mnemonic_bip49(
    phrase: &str,
    lang: &str,
    path: &BipPath,
) -> Res<XfrKeyPair, String> {
    restore_keypair_from_mnemonic_bip49_inner(phrase, lang, path).map_err(|e| genlog(e))
}

#[inline(always)]
fn restore_keypair_from_mnemonic_bip49_inner(
    phrase: &str,
    lang: &str,
    path: &BipPath,
) -> Result<XfrKeyPair> {
    restore_keypair_from_mnemonic!(phrase, lang, path, bip49).c(d!())
}

// check and generate a Language object from its string value.
#[inline(always)]
fn check_lang(lang: &str) -> Result<Language> {
    match lang {
        "en" => Ok(Language::English),
        "zh" => Ok(Language::ChineseSimplified),
        "zh_traditional" => Ok(Language::ChineseTraditional),
        "fr" => Ok(Language::French),
        "it" => Ok(Language::Italian),
        "ko" => Ok(Language::Korean),
        "sp" => Ok(Language::Spanish),
        "jp" => Ok(Language::Japanese),
        _ => Err(eg!("Unsupported language")),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn t_generate_mnemonic() {
        ["en", "zh", "zh_traditional", "fr", "it", "ko", "sp", "jp"]
            .iter()
            .for_each(|lang| {
                [12, 15, 18, 21, 24].iter().for_each(|wordslen| {
                    let phrase = generate_mnemonic_custom(*wordslen, lang).unwrap();
                    let path = BipPath {
                        coin: 917,
                        account: rand::random::<u32>() % 100,
                        change: rand::random::<u32>() % 100,
                        address: rand::random::<u32>() % 100,
                    };
                    assert_eq!(
                        *wordslen as usize,
                        phrase.split(" ").collect::<Vec<_>>().len()
                    );

                    pnk!(restore_keypair_from_mnemonic_bip44_inner(
                        &phrase, lang, &path
                    ));
                    pnk!(restore_keypair_from_mnemonic_bip49_inner(
                        &phrase, lang, &path
                    ));
                })
            });
    }

    #[test]
    fn t_generate_mnemonic_bad() {
        assert!(generate_mnemonic_custom(12, "xx").is_err());
        assert!(generate_mnemonic_custom(11, "zh").is_err());
        assert!(generate_mnemonic_custom(11, "xx").is_err());
    }
}
