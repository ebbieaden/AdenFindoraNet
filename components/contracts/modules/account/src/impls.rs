use crate::storage::*;
use crate::{App, Config};
use fp_core::{
    account::{MintOutput, SmartAccount},
    context::Context,
    crypto::Address,
    transaction::ActionResult,
};
use fp_traits::account::AccountAsset;
use ledger::data_model::ASSET_TYPE_FRA;
use ruc::*;
use std::collections::HashMap;
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
        } else if let Some(amount) = target_account.assets.get_mut(&asset) {
            *amount = (*amount).checked_add(balance).c(d!())?;
        } else {
            target_account.assets.insert(asset, balance);
        }

        AccountStore::insert(ctx.store.clone(), target, &target_account);
        Ok(())
    }

    fn burn(
        ctx: &Context,
        target: &Address,
        balance: u128,
        asset: AssetType,
    ) -> Result<()> {
        let mut target_account: SmartAccount =
            AccountStore::get(ctx.store.clone(), target)
                .c(d!("account does not exist"))?;
        if asset == ASSET_TYPE_FRA {
            target_account.balance = target_account
                .balance
                .checked_sub(balance)
                .c(d!("insufficient balance"))?;
        } else {
            if let Some(amount) = target_account.assets.get_mut(&asset) {
                *amount = (*amount)
                    .checked_sub(balance)
                    .c(d!("insufficient balance"))?;
            } else {
            }
            return Err(eg!("no this assets"));
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

impl<C: Config> App<C> {
    pub fn transfer_to_utxo(
        ctx: &Context,
        sender: Address,
        outputs: Vec<MintOutput>,
    ) -> Result<ActionResult> {
        let mut asset_amount = 0;
        let mut asset_map = HashMap::new();
        for output in &outputs {
            if output.asset == ASSET_TYPE_FRA {
                asset_amount += output.amount;
            } else if let Some(amount) = asset_map.get_mut(&output.asset) {
                *amount += output.amount;
            } else {
                asset_map.insert(output.asset, output.amount);
            }
        }

        log::debug!(target: "account", "this tx's amount is: FRA: {}, OTHER: {:?}", asset_amount, asset_map);

        let sa = Self::account_of(ctx, &sender).c(d!("no account!"))?;

        if sa.balance < asset_amount as u128 {
            return Err(eg!("insufficient balance fra"));
        }

        for (k, v) in asset_map.iter() {
            if let Some(asset_balance) = sa.assets.get(k) {
                if asset_balance < &(*v as u128) {
                    return Err(eg!("insufficient balance"));
                }
            } else {
                return Err(eg!("insufficient balance, no asset"));
            }
        }

        if asset_amount > 0 {
            Self::burn(ctx, &sender, asset_amount as u128, ASSET_TYPE_FRA)?;
        }

        for (k, v) in asset_map.into_iter() {
            Self::burn(ctx, &sender, v as u128, k)?;
        }
        Self::add_mint(ctx, outputs)?;
        Ok(ActionResult::default())
    }

    fn add_mint(ctx: &Context, mut outputs: Vec<MintOutput>) -> Result<()> {
        let ops = if let Some(mut ori_outputs) = MintOutputs::get(ctx.store.clone()) {
            ori_outputs.append(&mut outputs);
            ori_outputs
        } else {
            outputs
        };
        MintOutputs::put(ctx.store.clone(), ops);
        Ok(())
    }

    pub fn consume_mint(ctx: &Context, size: usize) -> Result<Vec<MintOutput>> {
        let res = if let Some(mut outputs) = MintOutputs::get(ctx.store.clone()) {
            if outputs.len() > size {
                let vec2 = outputs.split_off(size - outputs.len());
                MintOutputs::put(ctx.store.clone(), vec2);
            } else {
                MintOutputs::put(ctx.store.clone(), Vec::new());
            }
            outputs
        } else {
            Vec::new()
        };
        Ok(res)
    }
}
