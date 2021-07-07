use crate::Action;
use fp_core::{
    crypto::{Address, Signature},
    transaction,
};
use ruc::{eg, Result};

#[derive(Clone, PartialEq, Eq, Debug, Hash, Copy)]
pub enum RunTxMode {
    /// Check a transaction
    Check = 0,
    /// Recheck a (pending) transaction after a commit
    ReCheck = 1,
    /// Simulate a transaction
    Simulate = 2,
    /// Deliver a transaction
    Deliver = 3,
}

/// Unchecked transaction type as expected by this application.
pub type UncheckedTransaction =
    transaction::UncheckedTransaction<Address, Action, Signature>;

/// Transaction type that has already been checked.
pub type CheckedTransaction = transaction::CheckedTransaction<Address, Action>;

/// Convert base action to sub module action within CheckedTransaction
/// if tx is unsigned transaction.
pub fn convert_unsigned_transaction<A>(
    action: A,
    tx: CheckedTransaction,
) -> transaction::CheckedTransaction<Address, A> {
    transaction::CheckedTransaction {
        signed: tx.signed,
        function: action,
    }
}

/// Convert raw transaction to unchecked transaction.
pub fn convert_unchecked_transaction(
    transaction: &[u8],
) -> Result<UncheckedTransaction> {
    serde_json::from_slice::<UncheckedTransaction>(transaction).map_err(|e| eg!(e))
}

/// Convert raw ethereum transaction to unified format unchecked transaction.
pub fn convert_ethereum_transaction(transaction: &[u8]) -> Result<UncheckedTransaction> {
    let tx = serde_json::from_slice::<ethereum::Transaction>(transaction)
        .map_err(|e| eg!(e))?;
    Ok(UncheckedTransaction::new_unsigned(Action::Ethereum(
        module_ethereum::Action::Transact(tx),
    )))
}
