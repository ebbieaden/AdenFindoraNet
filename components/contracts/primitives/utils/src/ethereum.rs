use ethereum::{Transaction, TransactionAction, TransactionSignature};
use fp_core::crypto::Address;
use fp_traits::evm::{AddressMapping, EthereumAddressMapping};
use primitive_types::{H160, H256, U256};
use rlp::*;
use ruc::{eg, Result};
use sha3::{Digest, Keccak256};

pub struct KeyPair {
    pub address: H160,
    pub private_key: H256,
    pub account_id: Address,
}

pub fn generate_address(seed: u8) -> KeyPair {
    let private_key = H256::from_slice(&[(seed + 1) as u8; 32]);
    let secret_key = libsecp256k1::SecretKey::parse_slice(&private_key[..]).unwrap();
    let public_key =
        &libsecp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
    let address = H160::from(H256::from_slice(&Keccak256::digest(public_key)[..]));

    KeyPair {
        address,
        private_key,
        account_id: EthereumAddressMapping::into_account_id(address),
    }
}

pub fn sign_transaction_message(
    message: ethereum::TransactionMessage,
    private_key: &H256,
) -> Result<ethereum::Transaction> {
    let signing_message = libsecp256k1::Message::parse_slice(&message.hash()[..])
        .map_err(|_| eg!("invalid signing message"))?;
    let secret = &libsecp256k1::SecretKey::parse_slice(&private_key[..])
        .map_err(|_| eg!("invalid secret"))?;
    let (signature, recid) = libsecp256k1::sign(&signing_message, secret);

    let v = match message.chain_id {
        None => 27 + recid.serialize() as u64,
        Some(chain_id) => 2 * chain_id + 35 + recid.serialize() as u64,
    };
    let rs = signature.serialize();
    let r = H256::from_slice(&rs[0..32]);
    let s = H256::from_slice(&rs[32..64]);

    Ok(ethereum::Transaction {
        nonce: message.nonce,
        gas_price: message.gas_price,
        gas_limit: message.gas_limit,
        action: message.action,
        value: message.value,
        input: message.input.clone(),
        signature: ethereum::TransactionSignature::new(v, r, s)
            .ok_or(eg!("signer generated invalid signature"))?,
    })
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
