mod eth;
mod forward;

use baseapp::BaseApp;
use fp_utils::ethereum::generate_address;
use jsonrpc_core::{Error, ErrorCode};
use jsonrpc_http_server::{
    AccessControlAllowOrigin, DomainsValidation, RestApi, ServerBuilder,
};
use parking_lot::RwLock;
use std::sync::Arc;

pub use eth::{EthApiImpl, EthFilterApiImpl, NetApiImpl, Web3ApiImpl};
pub use fp_rpc_core::{EthApiServer, EthFilterApiServer, NetApiServer, Web3ApiServer};

fn internal_err<T: ToString>(message: T) -> Error {
    Error {
        code: ErrorCode::InternalError,
        message: message.to_string(),
        data: None,
    }
}

pub fn start_service(url: String, account_base_app: Arc<RwLock<BaseApp>>) {
    let mut io = jsonrpc_core::IoHandler::default();

    let signers = vec![generate_address(1)];
    io.extend_with(EthApiServer::to_delegate(EthApiImpl::new(
        url,
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
        .start_http(&"0.0.0.0:8545".parse().unwrap())
        .expect("Unable to start Ethereum api service");

    server.wait()
}
