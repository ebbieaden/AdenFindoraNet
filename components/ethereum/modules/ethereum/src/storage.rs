use ethereum::{Block, Receipt, Transaction};
use fp_core::context::*;
use fp_core::tuple_structs_deref;
use fp_evm::TransactionStatus;
pub use named_type::NamedType;
pub use named_type_derive::*;
use ruc::{d, Result, RucResult};
use serde::{Deserialize, Serialize};

/// Current building block's transactions and receipts.
#[derive(NamedType, Eq, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[named_type(short_prefix = "Ethereum_")]
pub struct Pending(Vec<(Transaction, TransactionStatus, Receipt)>);
tuple_structs_deref!(Pending, Vec<(Transaction, TransactionStatus, Receipt)>);

/// The current Ethereum block.
#[derive(NamedType, Eq, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[named_type(short_prefix = "Ethereum_")]
pub struct CurrentBlock(Option<Block>);
tuple_structs_deref!(CurrentBlock, Option<Block>);

/// The current Ethereum receipts.
#[derive(NamedType, Eq, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[named_type(short_prefix = "Ethereum_")]
pub struct CurrentReceipts(Option<Vec<Receipt>>);
tuple_structs_deref!(CurrentReceipts, Option<Vec<Receipt>>);

/// The current transaction statuses.
#[derive(NamedType, Eq, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[named_type(short_prefix = "Ethereum_")]
pub struct CurrentTransactionStatuses(Option<Vec<TransactionStatus>>);
tuple_structs_deref!(CurrentTransactionStatuses, Option<Vec<TransactionStatus>>);

pub fn get_pending(store: Arc<RwLock<CommitStore>>) -> Result<Pending> {
    let output = store.read().get(Pending::short_type_name().as_bytes())?;
    if let Some(val) = output {
        Ok(serde_json::from_slice::<Pending>(val.as_slice()).c(d!())?)
    } else {
        Ok(Default::default())
    }
}

pub fn set_pending(store: Arc<RwLock<CommitStore>>, pending: &Pending) -> Result<()> {
    let val = serde_json::to_vec(pending).c(d!())?;
    Ok(store
        .write()
        .set(Pending::short_type_name().as_bytes(), val))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_prefix_works() {
        assert_eq!(Pending::type_name(), "module_ethereum::storage::Pending");

        assert_eq!(Pending::short_type_name(), "Ethereum_Pending");
    }
}
