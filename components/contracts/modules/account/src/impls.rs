use crate::storage::*;
use crate::{App, Config};
use fp_core::{account::SmartAccount, context::Context, crypto::Address};
use ruc::{eg, Result};

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
}
