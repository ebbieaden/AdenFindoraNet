use ledger::data_model::AuthenticatedKVLookup as PlatformAuthenticatedKVLookup;
use std::ops::{Deref, DerefMut};
use zei::xfr::sig::{XfrKeyPair as ZeiXfrKeyPair, XfrPublicKey as ZeiXfrPublicKey};
use zei::xfr::structs::OpenAssetRecord as ZeiOpenAssetRecord;

pub struct AuthenticatedKVLookup(PlatformAuthenticatedKVLookup);

impl From<PlatformAuthenticatedKVLookup> for AuthenticatedKVLookup {
    fn from(v: PlatformAuthenticatedKVLookup) -> AuthenticatedKVLookup {
        AuthenticatedKVLookup(v)
    }
}

impl Deref for AuthenticatedKVLookup {
    type Target = PlatformAuthenticatedKVLookup;

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

pub struct XfrPublicKey(ZeiXfrPublicKey);

impl From<ZeiXfrPublicKey> for XfrPublicKey {
    fn from(v: ZeiXfrPublicKey) -> XfrPublicKey {
        XfrPublicKey(v)
    }
}

impl Deref for XfrPublicKey {
    type Target = ZeiXfrPublicKey;

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

#[derive(Clone)]
pub struct XfrKeyPair(ZeiXfrKeyPair);

impl From<ZeiXfrKeyPair> for XfrKeyPair {
    fn from(v: ZeiXfrKeyPair) -> XfrKeyPair {
        XfrKeyPair(v)
    }
}

impl Deref for XfrKeyPair {
    type Target = ZeiXfrKeyPair;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for XfrKeyPair {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct OpenAssetRecord(ZeiOpenAssetRecord);

impl From<ZeiOpenAssetRecord> for OpenAssetRecord {
    fn from(v: ZeiOpenAssetRecord) -> OpenAssetRecord {
        OpenAssetRecord(v)
    }
}

impl Deref for OpenAssetRecord {
    type Target = ZeiOpenAssetRecord;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OpenAssetRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
