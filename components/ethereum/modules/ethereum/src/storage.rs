use ethereum::{Block, Receipt, Transaction};
use fp_core::{storage::*, storage_wrapper};
use fp_evm::TransactionStatus;
use serde::{Deserialize, Serialize};

/// Current building block's transactions and receipts.
#[derive(NamedType, Eq, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[named_type(short_prefix = "Ethereum_")]
pub struct Pending(Vec<(Transaction, TransactionStatus, Receipt)>);
storage_wrapper!(Pending, Vec<(Transaction, TransactionStatus, Receipt)>);

/// The current Ethereum block.
#[derive(NamedType, Eq, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[named_type(short_prefix = "Ethereum_")]
pub struct CurrentBlock(Option<Block>);
storage_wrapper!(CurrentBlock, Option<Block>);

/// The current Ethereum receipts.
#[derive(NamedType, Eq, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[named_type(short_prefix = "Ethereum_")]
pub struct CurrentReceipts(Option<Vec<Receipt>>);
storage_wrapper!(CurrentReceipts, Option<Vec<Receipt>>);

/// The current transaction statuses.
#[derive(NamedType, Eq, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[named_type(short_prefix = "Ethereum_")]
pub struct CurrentTransactionStatuses(Option<Vec<TransactionStatus>>);
storage_wrapper!(CurrentTransactionStatuses, Option<Vec<TransactionStatus>>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_prefix_works() {
        assert_eq!(Pending::type_name(), "module_ethereum::storage::Pending");

        assert_eq!(Pending::short_type_name(), "Ethereum_Pending");
    }
}
