use ethereum::{Transaction, TransactionAction, TransactionSignature};
use parking_lot::RwLock;
use primitive_types::{H160, H256, U256};
use rlp::*;
use sha3::{Digest, Keccak256};
use std::{env::temp_dir, sync::Arc, time::SystemTime};
use storage::{db::FinDB, state::ChainState};

pub const CHAIN_ID: u64 = 523;

pub struct AccountInfo {
    pub address: H160,
    pub private_key: H256,
}

pub fn address_build(seed: u8) -> AccountInfo {
    //H256::from_low_u64_be((i + 1) as u64);
    let private_key = H256::from_slice(&[(seed + 1) as u8; 32]);
    let secret_key = libsecp256k1::SecretKey::parse_slice(&private_key[..]).unwrap();
    let public_key =
        &libsecp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
    let address = H160::from(H256::from_slice(&Keccak256::digest(public_key)[..]));

    AccountInfo {
        private_key,
        address,
    }
}

pub struct UnsignedTransaction {
    pub nonce: U256,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub action: TransactionAction,
    pub value: U256,
    pub input: Vec<u8>,
}

impl UnsignedTransaction {
    fn signing_rlp_append(&self, s: &mut RlpStream, chain_id: u64) {
        s.begin_list(9);
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas_limit);
        s.append(&self.action);
        s.append(&self.value);
        s.append(&self.input);
        s.append(&chain_id);
        s.append(&0u8);
        s.append(&0u8);
    }

    fn signing_hash(&self, chain_id: u64) -> H256 {
        let mut stream = RlpStream::new();
        self.signing_rlp_append(&mut stream, chain_id);
        H256::from_slice(&Keccak256::digest(&stream.out()).as_slice())
    }

    pub fn sign(&self, key: &H256, chain_id: u64) -> Transaction {
        let hash = self.signing_hash(chain_id);
        let msg = libsecp256k1::Message::parse(hash.as_fixed_bytes());
        let s = libsecp256k1::sign(
            &msg,
            &libsecp256k1::SecretKey::parse_slice(&key[..]).unwrap(),
        );
        let sig = s.0.serialize();

        let sig = TransactionSignature::new(
            s.1.serialize() as u64 % 2 + chain_id * 2 + 35,
            H256::from_slice(&sig[0..32]),
            H256::from_slice(&sig[32..64]),
        )
        .unwrap();

        Transaction {
            nonce: self.nonce,
            gas_price: self.gas_price,
            gas_limit: self.gas_limit,
            action: self.action,
            value: self.value,
            input: self.input.clone(),
            signature: sig,
        }
    }
}

pub fn create_temp_db() -> Arc<RwLock<ChainState<FinDB>>> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = temp_dir();
    path.push(format!("temp-findora-db–{}", time));
    let fdb = FinDB::open(path).unwrap();
    Arc::new(RwLock::new(ChainState::new(fdb, "temp_db".to_string())))
}
