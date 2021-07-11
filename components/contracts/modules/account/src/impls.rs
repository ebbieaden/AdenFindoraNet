use crate::storage::*;
use crate::{App, Config};
use fp_core::{account::SmartAccount, context::Context, crypto::Address};
use fp_traits::account::AccountAsset;
use ledger::data_model::ASSET_TYPE_FRA;
use ruc::*;
use zei::xfr::structs::AssetType;

impl<C: Config> AccountAsset<Address> for App<C> {
    fn account_of(ctx: &Context, who: &Address) -> Option<SmartAccount> {
        AccountStore::get(ctx.store.clone(), who)
    }

    fn balance(ctx: &Context, who: &Address) -> u128 {
        let who_account: SmartAccount =
            AccountStore::get(ctx.store.clone(), who).unwrap_or_default();
        who_account.balance
    }

    fn nonce(ctx: &Context, who: &Address) -> u64 {
        let who_account: SmartAccount =
            AccountStore::get(ctx.store.clone(), who).unwrap_or_default();
        who_account.nonce
    }

    fn inc_nonce(ctx: &Context, who: &Address) -> Result<u64> {
        let mut sa: SmartAccount =
            AccountStore::get(ctx.store.clone(), who).c(d!("account does not exist"))?;
        sa.nonce = sa.nonce.checked_add(1).c(d!("balance overflow"))?;
        AccountStore::insert(ctx.store.clone(), who, &sa);
        Ok(sa.nonce)
    }

    fn transfer(
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
                .c(d!("sender does not exist"))?;

        let mut to_account: SmartAccount =
            AccountStore::get(ctx.store.clone(), dest).unwrap_or_default();
        from_account.balance = from_account
            .balance
            .checked_sub(balance)
            .c(d!("insufficient balance"))?;
        to_account.balance = to_account
            .balance
            .checked_add(balance)
            .c(d!("balance overflow"))?;
        AccountStore::insert(ctx.store.clone(), sender, &from_account);
        AccountStore::insert(ctx.store.clone(), dest, &to_account);
        Ok(())
    }

    fn mint(
        ctx: &Context,
        target: &Address,
        balance: u128,
        asset: AssetType,
    ) -> Result<()> {
        let mut target_account: SmartAccount =
            AccountStore::get(ctx.store.clone(), target).unwrap_or_default();
        if asset == ASSET_TYPE_FRA {
            target_account.balance =
                target_account.balance.checked_add(balance).c(d!())?;
        } else {
            if let Some(amount) = target_account.assets.get_mut(&asset) {
                *amount = (*amount).checked_add(balance).c(d!())?;
            } else {
                target_account.assets.insert(asset, balance);
            }
        }
        AccountStore::insert(ctx.store.clone(), target, &target_account);
        Ok(())
    }

    fn withdraw(ctx: &Context, who: &Address, value: u128) -> Result<()> {
        let mut sa: SmartAccount =
            AccountStore::get(ctx.store.clone(), who).c(d!("account does not exist"))?;
        sa.balance = sa
            .balance
            .checked_sub(value)
            .c(d!("insufficient balance"))?;
        AccountStore::insert(ctx.store.clone(), who, &sa);
        Ok(())
    }

    fn refund(ctx: &Context, who: &Address, value: u128) -> Result<()> {
        let mut sa: SmartAccount =
            AccountStore::get(ctx.store.clone(), who).c(d!("account does not exist"))?;
        sa.balance = sa.balance.checked_add(value).c(d!("balance overflow"))?;
        AccountStore::insert(ctx.store.clone(), who, &sa);
        Ok(())
    }
}
