//!
//! # Staking
//!
//! Business logic based on [**Ledger Staking**](ledger::staking).
//!

use abci::{PubKey, ValidatorUpdate};
use ledger::staking::Staking;
use ruc::*;

// The top 50 candidate validators
// will become official validators.
const VALIDATOR_LIMIT: usize = 50;

/// Get the effective validators at current block height.
pub fn get_validators(staking: &Staking) -> Vec<ValidatorUpdate> {
    let mut vs = pnk!(staking.get_current_validators())
        .data
        .values()
        .map(|v| (v.td_power, &v.td_pubkey))
        .collect::<Vec<_>>();

    vs.sort_by_key(|v| -v.0);

    vs[..VALIDATOR_LIMIT]
        .iter()
        .map(|(power, pubkey)| {
            let mut vu = ValidatorUpdate::new();
            let mut pk = PubKey::new();
            // pk.set_field_type("ed25519".to_owned());
            pk.set_data(pubkey.to_vec());
            vu.set_power(*power);
            vu.set_pub_key(pk);
            vu
        })
        .collect()
}

#[cfg(test)]
mod test {
    #[test]
    fn demo() {}
}
