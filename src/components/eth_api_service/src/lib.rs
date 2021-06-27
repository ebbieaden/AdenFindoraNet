#![deny(warnings)]

pub mod service {
    use fc_rpc::EthApiImpl;
    use fc_rpc_core::EthApiServer;

    pub fn run() {
	    let mut io = jsonrpc_core::IoHandler::new();
	    io.extend_with(EthApiServer::to_delegate(EthApiImpl::new()));
    }
}

