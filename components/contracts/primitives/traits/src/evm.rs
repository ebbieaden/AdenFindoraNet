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

pub trait DecimalsMapping {
    fn from_native_token(balance: U256) -> Option<U256>;

    fn into_native_token(balance: U256) -> U256;
}

/// FRA decimals
const FRA_DECIMALS: u32 = 6;

/// ETH decimals
const ETH_DECIMALS: u32 = 18;

/// Ethereum decimals mapping.
pub struct EthereumDecimalsMapping;

impl DecimalsMapping for EthereumDecimalsMapping {
    fn from_native_token(balance: U256) -> Option<U256> {
        balance.checked_mul(U256::from(10_u64.pow(ETH_DECIMALS - FRA_DECIMALS)))
    }

    fn into_native_token(balance: U256) -> U256 {
        balance
            .checked_div(U256::from(10_u64.pow(ETH_DECIMALS - FRA_DECIMALS)))
            .unwrap_or(U256::zero())
    }
}

/// Trait that outputs the current transaction gas price.
pub trait FeeCalculator {
    /// Return the minimal required gas price.
    fn min_gas_price() -> U256;
}

impl FeeCalculator for () {
    fn min_gas_price() -> U256 {
        // 1000 GWEI
        U256::from(1_0000_0000_0000_u64)
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
