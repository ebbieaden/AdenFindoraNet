use attohttpc::Response;
use baseapp::UncheckedTransaction;
use ruc::{d, Result, RucResult};

// The call will not wait for the execution result of the transaction and
// will return immediately after submission.
#[allow(unused)]
pub const TX_ASYNC: &str = "broadcast_tx_async";
// The call submits the broadcast transaction synchronously and
// waits for the response result of CheckTx.
#[allow(unused)]
pub const TX_SYNC: &str = "broadcast_tx_sync";
// The call will broadcast the transaction and
// wait for the submission result to return.
#[allow(unused)]
pub const TX_COMMIT: &str = "broadcast_tx_commit";

pub struct TendermintForward {
    url: String,
}

impl TendermintForward {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl TendermintForward {
    pub fn forward_txn(
        &self,
        txn: UncheckedTransaction,
        tx_mode: &str,
    ) -> Result<Response> {
        let txn_json = serde_json::to_string(&txn).c(d!())?;
        let txn_b64 = base64::encode_config(&txn_json.as_str(), base64::URL_SAFE);

        let body = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":\"anything\",\"method\":\"{}\",\"params\": {{\"tx\": \"{}\"}}}}",
            tx_mode, txn_b64
        );

        attohttpc::post(self.url.as_str())
            .header(attohttpc::header::CONTENT_TYPE, "application/json")
            .text(body)
            .send()
            .c(d!())
    }
}
