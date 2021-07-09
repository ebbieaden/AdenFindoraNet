mod eth;
pub use eth::{EthApiImpl, NetApiImpl, Web3ApiImpl};
pub use fc_rpc_core::{EthApiServer, NetApiServer, Web3ApiServer};
use jsonrpc_core::{Error, ErrorCode};
use jsonrpc_http_server::{
    AccessControlAllowOrigin, DomainsValidation, RestApi, ServerBuilder,
};

fn internal_err<T: ToString>(message: T) -> Error {
    Error {
        code: ErrorCode::InternalError,
        message: message.to_string(),
        data: None,
    }
}

pub fn start_service() {
    let mut io = jsonrpc_core::IoHandler::default();
    io.extend_with(EthApiServer::to_delegate(EthApiImpl::new()));
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
