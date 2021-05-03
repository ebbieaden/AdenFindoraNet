use ledger::data_model::Transaction;
use ruc::*;
use submission_server::TxnForward;

pub struct TendermintForward {
    pub tendermint_reply: String,
}

impl AsRef<str> for TendermintForward {
    fn as_ref(&self) -> &str {
        self.tendermint_reply.as_str()
    }
}

impl TxnForward for TendermintForward {
    fn forward_txn(&self, _: Transaction) -> Result<()> {
        Ok(())
    }
}
