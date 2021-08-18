use abciapp::abci;

fn main() {
    utils::logging::init_logging(None);

    abci::run().unwrap();
}
