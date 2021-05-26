use super::data_model::{
    ClientAssetRecord as RawClientAssetRecord, OwnerMemo as RawOwnerMemo,
    TxoRef as RawTxoRef,
};
use ledger::data_model::AuthenticatedKVLookup as AuthKVLookup;
use std::ops::{Deref, DerefMut};
use zei::xfr::sig::XfrKeyPair as RawXfrKeyPair;
use zei::xfr::sig::XfrPublicKey as PublicKey;

pub struct AuthenticatedKVLookup(AuthKVLookup);

impl From<AuthKVLookup> for AuthenticatedKVLookup {
    fn from(v: AuthKVLookup) -> AuthenticatedKVLookup {
        AuthenticatedKVLookup(v)
    }
}

impl Deref for AuthenticatedKVLookup {
    type Target = AuthKVLookup;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AuthenticatedKVLookup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct XfrPublicKey(PublicKey);

impl From<PublicKey> for XfrPublicKey {
    fn from(v: PublicKey) -> XfrPublicKey {
        XfrPublicKey(v)
    }
}

impl Deref for XfrPublicKey {
    type Target = PublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for XfrPublicKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone)]
pub struct TxoRef(RawTxoRef);

impl From<RawTxoRef> for TxoRef {
    fn from(v: RawTxoRef) -> TxoRef {
        TxoRef(v)
    }
}

impl Deref for TxoRef {
    type Target = RawTxoRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TxoRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

////////////////////////////////////////////////////////////////////////////////
#[derive(Clone)]
pub struct ClientAssetRecord(RawClientAssetRecord);

impl From<RawClientAssetRecord> for ClientAssetRecord {
    fn from(v: RawClientAssetRecord) -> ClientAssetRecord {
        ClientAssetRecord(v)
    }
}

impl Deref for ClientAssetRecord {
    type Target = RawClientAssetRecord;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ClientAssetRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct OwnerMemo(RawOwnerMemo);

impl From<RawOwnerMemo> for OwnerMemo {
    fn from(v: RawOwnerMemo) -> OwnerMemo {
        OwnerMemo(v)
    }
}

impl Deref for OwnerMemo {
    type Target = RawOwnerMemo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OwnerMemo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct XfrKeyPair(RawXfrKeyPair);

impl From<RawXfrKeyPair> for XfrKeyPair {
    fn from(v: RawXfrKeyPair) -> XfrKeyPair {
        XfrKeyPair(v)
    }
}

impl Deref for XfrKeyPair {
    type Target = RawXfrKeyPair;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for XfrKeyPair {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
