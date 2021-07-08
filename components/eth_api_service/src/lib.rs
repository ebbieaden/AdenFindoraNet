pub mod service {
    use fc_rpc::{EthApiImpl, NetApiImpl, Web3ApiImpl};
    use fc_rpc_core::{EthApiServer, NetApiServer, Web3ApiServer};
    use jsonrpc_http_server::{
        AccessControlAllowOrigin, DomainsValidation, RestApi, ServerBuilder,
    };

    pub fn start() {
        let mut io = jsonrpc_core::IoHandler::default();
        io.extend_with(EthApiServer::to_delegate(EthApiImpl::new()));
        io.extend_with(NetApiServer::to_delegate(NetApiImpl::new()));
        io.extend_with(Web3ApiServer::to_delegate(Web3ApiImpl::new()));

        let server = ServerBuilder::new(io)
            .threads(1)
            .rest_api(RestApi::Secure)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Any,
            ]))
            .start_http(&"0.0.0.0:8545".parse().unwrap())
            .expect("Unable to start eth api server");

        server.wait()
    }
}
