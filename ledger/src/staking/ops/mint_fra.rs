//!
//! # Mint FRA
//!
//! A more standard CoinBase implementation.
//!

use crate::{
    data_model::{TxOutput, ASSET_TYPE_FRA},
    staking::{Amount, FRA},
};
use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;
use serde::{Deserialize, Serialize};
use zei::xfr::sig::XfrPublicKey;
use zei::{
    setup::PublicParams,
    xfr::{
        asset_record::{build_blind_asset_record, AssetRecordType},
        structs::AssetRecordTemplate,
    },
};

/// 420 million FRAs
pub const MINT_AMOUNT_LIMIT: Amount = 420 * 100_0000 * FRA;

#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MintFraOps {
    pub entries: Vec<MintEntry>,
}

impl MintFraOps {
    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new(entries: Vec<MintEntry>) -> Self {
        MintFraOps { entries }
    }

    #[inline(always)]
    #[allow(missing_docs)]
    pub fn get_related_pubkeys(&self) -> Vec<XfrPublicKey> {
        self.entries.iter().map(|e| e.target_pk).collect()
    }
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MintEntry {
    pub kind: MintKind,
    pub target_pk: XfrPublicKey,
    pub amount: Amount,
    pub utxo: TxOutput,
}

impl MintEntry {
    #[inline(always)]
    #[allow(missing_docs)]
    pub fn new(kind: MintKind, target_pk: XfrPublicKey, amount: Amount) -> Self {
        let mut prng = ChaChaRng::seed_from_u64(0);
        let ar = AssetRecordTemplate::with_no_asset_tracing(
            amount,
            ASSET_TYPE_FRA,
            AssetRecordType::NonConfidentialAmount_NonConfidentialAssetType,
            target_pk,
        );
        let pc_gens = PublicParams::default().pc_gens;
        let (ba, _, _) = build_blind_asset_record(&mut prng, &pc_gens, &ar, vec![]);

        let utxo = TxOutput {
            id: None,
            record: ba,
            lien: None,
        };

        MintEntry {
            kind,
            target_pk,
            amount,
            utxo,
        }
    }
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MintKind {
    Claim,
    UnStake,
}
