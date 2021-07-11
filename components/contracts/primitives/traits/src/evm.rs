use fp_core::{
    context::Context,
    crypto::{Address, Address32},
};
use primitive_types::{H160, U256};
use ruc::Result;
use std::convert::TryFrom;

pub trait AddressMapping {
    fn into_account_id(address: H160) -> Address;
}

/// Ethereum address mapping.
pub struct EthereumAddressMapping;

impl AddressMapping for EthereumAddressMapping {
    fn into_account_id(address: H160) -> Address {
        let mut data = [0u8; 32];
        data[0..20].copy_from_slice(&address[..]);
        Address32::try_from(&data[..]).unwrap()
    }
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
