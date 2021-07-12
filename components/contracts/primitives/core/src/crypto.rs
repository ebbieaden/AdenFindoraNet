use ruc::eg;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use zei::serialization::ZeiFromToBytes;
use zei::xfr::sig::{XfrPublicKey, XfrSignature};
use ledger::address::SmartAddress;

/// An opaque 32-byte cryptographic identifier.
#[derive(
    Clone, Eq, PartialEq, Ord, PartialOrd, Default, Hash, Serialize, Deserialize, Debug,
)]
pub struct Address32([u8; 32]);

impl AsRef<[u8]> for Address32 {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl From<XfrPublicKey> for Address32 {
    fn from(k: XfrPublicKey) -> Self {
        Address32::try_from(k.zei_to_bytes().as_slice()).unwrap()
    }
}

impl From<SmartAddress> for Address32 {
    fn from(addr: SmartAddress) -> Self {
        match addr {
            SmartAddress::Ethereum(a) => {
                let mut data = [0u8; 32];
                data[0..20].copy_from_slice(&a.0[..]);
                Address32::try_from(&data[..]).unwrap()
            },
            SmartAddress::Xfr(a) => Self::from(a),
            SmartAddress::Other => Self([0u8; 32]),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a [u8]> for Address32 {
    type Error = ();
    fn try_from(x: &'a [u8]) -> Result<Address32, ()> {
        if x.len() == 32 {
            let mut r = Address32::default();
            r.0.copy_from_slice(x);
            Ok(r)
        } else {
            Err(())
        }
    }
}

/// Some type that is able to be collapsed into an account ID. It is not possible to recreate the
/// original value from the account ID.
pub trait IdentifyAccount {
    /// The account ID that this can be transformed into.
    type AccountId;
    /// Transform into an account.
    fn into_account(self) -> Self::AccountId;
}

/// Means of signature verification.
pub trait Verify {
    /// Type of the signer.
    type Signer: IdentifyAccount;
    /// Verify a signature.
    ///
    /// Return `true` if signature is valid for the value.
    fn verify(
        &self,
        msg: &[u8],
        signer: &<Self::Signer as IdentifyAccount>::AccountId,
    ) -> bool;
}

/// Signature verify that can work with any known signature types..
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

impl Verify for MultiSignature {
    type Signer = MultiSigner;

    fn verify(&self, msg: &[u8], signer: &Address32) -> bool {
        match self {
            Self::Xfr(ref sig) => {
                if let Ok(who) = XfrPublicKey::zei_from_bytes(signer.as_ref()) {
                    sig.verify(msg, &who)
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MultiSigner {
    /// An zei xfr identity.
    Xfr(XfrPublicKey),
}

impl Default for MultiSigner {
    fn default() -> Self {
        Self::Xfr(Default::default())
    }
}

impl From<XfrPublicKey> for MultiSigner {
    fn from(x: XfrPublicKey) -> Self {
        Self::Xfr(x)
    }
}

impl TryFrom<MultiSigner> for XfrPublicKey {
    type Error = ();
    fn try_from(m: MultiSigner) -> Result<Self, Self::Error> {
        let MultiSigner::Xfr(x) = m;
        Ok(x)
    }
}

impl IdentifyAccount for XfrPublicKey {
    type AccountId = Self;
    fn into_account(self) -> Self {
        self
    }
}

impl IdentifyAccount for MultiSigner {
    type AccountId = Address32;
    fn into_account(self) -> Address32 {
        match self {
            MultiSigner::Xfr(who) => who.into(),
        }
    }
}

impl Verify for XfrSignature {
    type Signer = XfrPublicKey;

    fn verify(&self, msg: &[u8], signer: &XfrPublicKey) -> bool {
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
pub fn secp256k1_ecdsa_recover(sig: &[u8; 65], msg: &[u8; 32]) -> ruc::Result<[u8; 64]> {
    let rs = libsecp256k1::Signature::parse_standard_slice(&sig[0..64])
        .map_err(|_| eg!("Ecdsa signature verify error: bad RS"))?;
    let v =
        libsecp256k1::RecoveryId::parse(
            if sig[64] > 26 { sig[64] - 27 } else { sig[64] } as u8,
        )
        .map_err(|_| eg!("Ecdsa signature verify error: bad V"))?;
    let pubkey = libsecp256k1::recover(&libsecp256k1::Message::parse(msg), &rs, &v)
        .map_err(|_| eg!("Ecdsa signature verify error: bad signature"))?;
    let mut res = [0u8; 64];
    res.copy_from_slice(&pubkey.serialize()[1..65]);
    Ok(res)
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type Address = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::rand_core::SeedableRng;
    use rand_chacha::ChaChaRng;
    use zei::xfr::sig::XfrKeyPair;

    #[test]
    fn xfr_sign_verify_work() {
        let mut prng = ChaChaRng::from_entropy();
        let alice = XfrKeyPair::generate(&mut prng);
        let sig = alice.get_sk_ref().sign(b"hello", alice.get_pk_ref());
        let signer = MultiSigner::from(alice.get_pk());
        let sig = MultiSignature::from(sig);

        assert!(
            sig.verify(b"hello", &signer.into_account()),
            "signature verify failed"
        );
    }
}
