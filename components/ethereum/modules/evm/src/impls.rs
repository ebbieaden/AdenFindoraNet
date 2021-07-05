use crate::storage::*;
use crate::{App, Config};
use fp_core::{context::Context, crypto::Address};
use fp_evm::{
    traits::{AddressMapping, OnChargeEVMTransaction},
    Account,
};
use primitive_types::{H160, U256};
use ruc::Result;

impl<C: Config> App<C> {
    /// Check whether an account is empty.
    pub fn is_account_empty(ctx: &Context, address: &H160) -> bool {
        let account = Self::account_basic(address);
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
    pub fn account_basic(address: &H160) -> Account {
        let _account_id = C::AddressMapping::into_account_id(*address);

        // TODO
        let nonce = U256::zero();
        let balance = U256::zero();
        // let nonce = frame_system::Module::<T>::account_nonce(&account_id);
        // let balance = T::Currency::free_balance(&account_id);

        Account { nonce, balance }
    }

    /// Get the block proposer.
    pub fn find_proposer(_ctx: &Context) -> H160 {
        todo!()
    }
}

/// Ethereum address mapping.
pub struct EthereumAddressMapping;

impl AddressMapping for EthereumAddressMapping {
    fn into_account_id(_address: H160) -> Address {
        todo!()
    }
}

/// Implements the transaction payment for a module implementing the `Currency`
/// trait (eg. the pallet_balances) using an unbalance handler (implementing
/// `OnUnbalanced`).
impl<C: Config> OnChargeEVMTransaction for App<C> {
    type LiquidityInfo = ();

    fn withdraw_fee(_who: &H160, _fee: U256) -> Result<Self::LiquidityInfo> {
        todo!()
    }

    fn correct_and_deposit_fee(
        _who: &H160,
        _corrected_fee: U256,
        _already_withdrawn: Self::LiquidityInfo,
    ) -> Result<()> {
        todo!()
    }
}
