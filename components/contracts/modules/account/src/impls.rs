use crate::storage::*;
use crate::{App, Config};
use fp_core::{account::SmartAccount, context::Context, crypto::Address};
use ledger::data_model::ASSET_TYPE_FRA;
use ruc::*;
use zei::xfr::structs::AssetType;

impl<C: Config> App<C> {
    // Transfer some balance from `sender` to `dest`
    pub fn do_transfer(
        ctx: &Context,
        sender: &Address,
        dest: &Address,
        balance: u128,
    ) -> Result<()> {
        if balance == 0 || sender == dest {
            return Ok(());
        }
        let mut from_account: SmartAccount =
            AccountStore::get(ctx.store.clone(), sender)
                .ok_or(eg!("sender does not exist"))?;

        let mut to_account: SmartAccount =
            AccountStore::get(ctx.store.clone(), sender).unwrap_or_default();
        from_account.balance = from_account
            .balance
            .checked_sub(balance)
            .ok_or(eg!("insufficient balance"))?;
        to_account.balance = to_account
            .balance
            .checked_add(balance)
            .ok_or(eg!("balance overflow"))?;
        Ok(())
    }

    pub fn mint_balance(
        ctx: &Context,
        target: &Address,
        balance: u128,
        asset: AssetType,
    ) -> Result<()> {
        let mut target_account: SmartAccount =
            AccountStore::get(ctx.store.clone(), target).c(d!())?;
        if asset == ASSET_TYPE_FRA {
            target_account.balance.checked_add(balance).c(d!())?;
        } else {
            if let Some(amount) = target_account.assets.get_mut(&asset) {
                (*amount).checked_add(balance).c(d!())?;
            } else {
                target_account.assets.insert(asset, balance);
            }
        }
        Ok(())
    }

    //     // This function need add with transfer to utxo
    // pub fn burn_balance(ctx: &Context, target: &Address, balance: u128, asset: AssetType) -> Result<()> {
    //     let target_account: SmartAccount =
    //         AccountStore::get(ctx.store.clone(), target).c(d!())?;
    //     target_account.balance.checked_sub(balance).c(d!())?;
    //     Ok(())
    // }
}
