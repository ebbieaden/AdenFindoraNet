mod eth;

use baseapp::BaseApp;
use evm::{ExitError, ExitReason};
use fp_utils::ecdsa::SecpPair;
use jsonrpc_core::{Error, ErrorCode};
use jsonrpc_http_server::{
    AccessControlAllowOrigin, DomainsValidation, RestApi, ServerBuilder,
};
use parking_lot::RwLock;
use serde_json::Value;
use std::sync::Arc;

pub use eth::{EthApiImpl, EthFilterApiImpl, NetApiImpl, Web3ApiImpl};
pub use fp_rpc_core::{EthApiServer, EthFilterApiServer, NetApiServer, Web3ApiServer};
use log::error;
use rustc_hex::ToHex;

pub fn start_service(
    url_evm: String,
    url_tdmt: String,
    account_base_app: Arc<RwLock<BaseApp>>,
) {
    let mut io = jsonrpc_core::IoHandler::default();

    // PrivateKey: 9f7bebaa5c55464b10150bc2e0fd552e915e2bdbca95cc45ed1c909aca96e7f5
    // Address: 0xf6aca39539374993b37d29ccf0d93fa214ea0af1
    let dev_signer = "zebra paddle unveil toilet weekend space gorilla lesson relief useless arrive picture";
    let signers = vec![SecpPair::from_phrase(dev_signer, None).unwrap().0];
    io.extend_with(EthApiServer::to_delegate(EthApiImpl::new(
        url_tdmt,
        account_base_app,
        signers,
    )));
    io.extend_with(NetApiServer::to_delegate(NetApiImpl::new()));
    io.extend_with(Web3ApiServer::to_delegate(Web3ApiImpl::new()));

    let server = ServerBuilder::new(io)
        .threads(2)
        .rest_api(RestApi::Secure)
        .cors(DomainsValidation::AllowOnly(vec![
            AccessControlAllowOrigin::Any,
        ]))
        .start_http(&url_evm.parse().unwrap())
        .expect("Unable to start Ethereum api service");

    server.wait()
}

pub fn internal_err<T: ToString>(message: T) -> Error {
    error!(target: "eth_rpc", "internal error: {:?}", message.to_string());
    Error {
        code: ErrorCode::InternalError,
        message: message.to_string(),
        data: None,
    }
}

pub fn error_on_execution_failure(
    reason: &ExitReason,
    data: &[u8],
) -> Result<(), Error> {
    match reason {
        ExitReason::Succeed(_) => Ok(()),
        ExitReason::Error(e) => {
            if *e == ExitError::OutOfGas {
                // `ServerError(0)` will be useful in estimate gas
                return Err(Error {
                    code: ErrorCode::ServerError(0),
                    message: "out of gas".to_string(),
                    data: None,
                });
            }
            Err(Error {
                code: ErrorCode::InternalError,
                message: format!("evm error: {:?}", e),
                data: Some(Value::String("0x".to_string())),
            })
        }
        ExitReason::Revert(_) => {
            let mut message =
                "VM Exception while processing transaction: revert".to_string();
            // A minimum size of error function selector (4) + offset (32) + string length (32)
            // should contain a utf-8 encoded revert reason.
            if data.len() > 68 {
                let message_len = data[36..68].iter().sum::<u8>();
                let body: &[u8] = &data[68..68 + message_len as usize];
                if let Ok(reason) = std::str::from_utf8(body) {
                    message = format!("{} {}", message, reason.to_string());
                }
            }
            Err(Error {
                code: ErrorCode::InternalError,
                message,
                data: Some(Value::String(data.to_hex())),
            })
        }
        ExitReason::Fatal(e) => Err(Error {
            code: ErrorCode::InternalError,
            message: format!("evm fatal: {:?}", e),
            data: Some(Value::String("0x".to_string())),
        }),
    }
}
