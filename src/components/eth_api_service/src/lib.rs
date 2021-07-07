pub mod service {
    use fc_rpc::EthApiImpl;
    use fc_rpc_core::EthApiServer;
    use jsonrpc_http_server::{
        AccessControlAllowOrigin, DomainsValidation, RestApi, ServerBuilder,
    };
    use std::thread;

    pub fn start() {
        let mut io = jsonrpc_core::IoHandler::default();
        io.extend_with(EthApiServer::to_delegate(EthApiImpl::new()));

        let server = ServerBuilder::new(io)
            .threads(2)
            .rest_api(RestApi::Secure)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Any,
            ]))
            .start_http(&"0.0.0.0:7545".parse().unwrap())
            .expect("Unable to start eth api server");

        thread::spawn(move || {
            server.wait();
        });
    }
}
