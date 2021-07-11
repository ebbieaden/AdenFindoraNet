use crate::storage::*;
use crate::{App, Config};
use fp_core::context::Context;
use fp_evm::Account;
use fp_traits::{
    account::AccountAsset,
    evm::{AddressMapping, OnChargeEVMTransaction},
};
use primitive_types::{H160, U256};
use ruc::Result;

impl<C: Config> App<C> {
    /// Check whether an account is empty.
    pub fn is_account_empty(ctx: &Context, address: &H160) -> bool {
        let account = Self::account_basic(ctx, address);
        let code_len = AccountCodes::decode_len(ctx.store.clone(), address).unwrap_or(0);

        account.nonce == U256::zero() && account.balance == U256::zero() && code_len == 0
    }

    /// Remove an account.
    pub fn remove_account(ctx: &Context, address: &H160) {
        AccountCodes::remove(ctx.store.clone(), address);
        AccountStorages::remove_prefix(ctx.store.clone(), address);
    }

    /// Create an account.
    pub fn create_account(ctx: &Context, address: H160, code: Vec<u8>) {
        if code.is_empty() {
            return;
        }

        AccountCodes::insert(ctx.store.clone(), &address, &code);
    }

    /// Get the account basic in EVM format.
    pub fn account_basic(ctx: &Context, address: &H160) -> Account {
        let account_id = C::AddressMapping::into_account_id(*address);
        let nonce = U256::from(C::AccountAsset::nonce(ctx, &account_id));
        let balance = U256::from(C::AccountAsset::balance(ctx, &account_id));

        Account { nonce, balance }
    }

    /// Get the block proposer.
    pub fn find_proposer(_ctx: &Context) -> H160 {
        todo!()
    }
}

/// Implements the transaction payment for a module implementing the `Currency`
/// trait (eg. the pallet_balances) using an unbalance handler (implementing
/// `OnUnbalanced`).
impl<C: Config> OnChargeEVMTransaction for App<C> {
    fn withdraw_fee(ctx: &Context, who: &H160, fee: U256) -> Result<()> {
        let account_id = C::AddressMapping::into_account_id(*who);
        C::AccountAsset::withdraw(ctx, &account_id, fee.low_u128())
    }

    fn correct_and_deposit_fee(
        ctx: &Context,
        who: &H160,
        corrected_fee: U256,
        already_withdrawn: U256,
    ) -> Result<()> {
        let account_id = C::AddressMapping::into_account_id(*who);
        let refund_amount = already_withdrawn.saturating_sub(corrected_fee);
        C::AccountAsset::refund(ctx, &account_id, refund_amount.low_u128())
    }
}
