use ruc::eg;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use zei::xfr::sig::{XfrPublicKey, XfrSignature};

/// Signature verify that can work with any known signature types..
#[derive(Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum MultiSignature {
    /// An zei xfr signature.
    Xfr(XfrSignature),
}

impl From<XfrSignature> for MultiSignature {
    fn from(x: XfrSignature) -> Self {
        MultiSignature::Xfr(x)
    }
}

impl TryFrom<MultiSignature> for XfrSignature {
    type Error = ();
    fn try_from(m: MultiSignature) -> Result<Self, Self::Error> {
        match m {
            MultiSignature::Xfr(x) => Ok(x),
        }
    }
}

/// Means of signature verification.
pub trait Verify {
    /// Type of the signer.
    type Signer;
    /// Verify a signature.
    ///
    /// Return `true` if signature is valid for the value.
    fn verify(&self, msg: &[u8], signer: &Self::Signer) -> bool;
}

impl Verify for XfrSignature {
    type Signer = XfrPublicKey;

    fn verify(&self, msg: &[u8], signer: &Self::Signer) -> bool {
        signer.verify(msg, self).is_ok()
    }
}

/// Verify and recover a SECP256k1 ECDSA signature.
///
/// - `sig` is passed in RSV format. V should be either `0/1` or `27/28`.
/// - `msg` is the blake2-256 hash of the message.
///
/// Returns `Err` if the signature is bad, otherwise the 64-byte pubkey
/// (doesn't include the 0x04 prefix).
fn secp256k1_ecdsa_recover(sig: &[u8; 65], msg: &[u8; 32]) -> ruc::Result<[u8; 64]> {
    let rs = secp256k1::Signature::parse_slice(&sig[0..64])
        .map_err(|_| eg!("Ecdsa signature verify error: bad RS"))?;
    let v =
        secp256k1::RecoveryId::parse(
            if sig[64] > 26 { sig[64] - 27 } else { sig[64] } as u8
        )
        .map_err(|_| eg!("Ecdsa signature verify error: bad V"))?;
    let pubkey = secp256k1::recover(&secp256k1::Message::parse(msg), &rs, &v)
        .map_err(|_| eg!("Ecdsa signature verify error: bad signature"))?;
    let mut res = [0u8; 64];
    res.copy_from_slice(&pubkey.serialize()[1..65]);
    Ok(res)
}
