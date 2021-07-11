use fp_core::{context::Context, crypto::Address};
use primitive_types::{H160, U256};
use ruc::Result;

pub trait AddressMapping {
    fn into_account_id(address: H160) -> Address;
}

/// Trait that outputs the current transaction gas price.
pub trait FeeCalculator {
    /// Return the minimal required gas price.
    fn min_gas_price() -> U256;
}

impl FeeCalculator for () {
    fn min_gas_price() -> U256 {
        U256::zero()
    }
}

/// Handle withdrawing, refunding and depositing of transaction fees.
pub trait OnChargeEVMTransaction {
    /// Before the transaction is executed the payment of the transaction fees
    /// need to be secured.
    fn withdraw_fee(ctx: &Context, who: &H160, fee: U256) -> Result<()>;

    /// After the transaction was executed the actual fee can be calculated.
    /// This function should refund any overpaid fees.
    fn correct_and_deposit_fee(
        ctx: &Context,
        who: &H160,
        corrected_fee: U256,
        already_withdrawn: U256,
    ) -> Result<()>;
}
