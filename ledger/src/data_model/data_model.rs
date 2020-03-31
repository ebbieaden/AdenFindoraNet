#![deny(warnings)]
use super::errors;
use crate::policy_script::{Policy, PolicyGlobals, TxnPolicyData};
use bitmap::SparseMap;
use chrono::prelude::*;
use cryptohash::sha256::Digest as BitDigest;
use cryptohash::{sha256, HashValue, Proof};
use errors::PlatformError;
use itertools::Itertools;
use rand_chacha::ChaChaRng;
use rand_core::{CryptoRng, RngCore, SeedableRng};
use std::boxed::Box;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use zei::xfr::lib::gen_xfr_body;
use zei::xfr::sig::{XfrKeyPair, XfrPublicKey, XfrSecretKey, XfrSignature};
use zei::xfr::structs::{AssetRecord, BlindAssetRecord, OpenAssetRecord, XfrBody};

pub fn b64enc<T: ?Sized + AsRef<[u8]>>(input: &T) -> String {
  base64::encode_config(input, base64::URL_SAFE)
}
pub fn b64dec<T: ?Sized + AsRef<[u8]>>(input: &T) -> Result<Vec<u8>, base64::DecodeError> {
  base64::decode_config(input, base64::URL_SAFE)
}

// Unique Identifier for ledger objects
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Code {
  pub val: [u8; 16],
}

pub type AssetTypeCode = Code;
pub type AssetPolicyKey = Code;
pub type SmartContractKey = Code;

impl Code {
  pub fn gen_random() -> Self {
    let mut small_rng = ChaChaRng::from_entropy();
    let mut buf: [u8; 16] = [0u8; 16];
    small_rng.fill_bytes(&mut buf);
    Self { val: buf }
  }
  pub fn new_from_str(s: &str) -> Self {
    let mut as_vec = s.to_string().into_bytes();
    as_vec.resize(16, 0u8);
    let buf = <[u8; 16]>::try_from(as_vec.as_slice()).unwrap();
    Self { val: buf }
  }
  pub fn new_from_base64(b64: &str) -> Result<Self, PlatformError> {
    if let Ok(mut bin) = b64dec(b64) {
      bin.resize(16, 0u8);
      let buf = <[u8; 16]>::try_from(bin.as_slice()).unwrap();
      Ok(Self { val: buf })
    } else {
      Err(PlatformError::DeserializationError)
    }
  }
  pub fn to_base64(&self) -> String {
    b64enc(&self.val)
  }
}

// Wrapper around a serialized variable that maintains type semantics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Serialized<T> {
  pub val: String,
  phantom: PhantomData<T>,
}

impl<T> Default for Serialized<T> where T: Default + serde::Serialize + serde::de::DeserializeOwned
{
  fn default() -> Self {
    Self::new(&T::default())
  }
}

impl<T> Serialized<T> where T: serde::Serialize + serde::de::DeserializeOwned
{
  pub fn new(to_serialize: &T) -> Self {
    Serialized { val: b64enc(&bincode::serialize(&to_serialize).unwrap()),
                 phantom: PhantomData }
  }

  pub fn deserialize(&self) -> T {
    bincode::deserialize(&b64dec(&self.val).unwrap()).unwrap()
  }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct AssetDigest {
  // Generated from the asset definition, also unique
  pub val: [u8; 32],
}

// TODO: Define Memo
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Memo(pub String);
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ConfidentialMemo;
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Commitment([u8; 32]);

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct XfrAddress {
  pub key: XfrPublicKey,
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for XfrAddress {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.key.as_bytes().hash(state);
  }
}

// TODO(joe): Better name! There's more than one thing that gets issued.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IssuerPublicKey {
  pub key: XfrPublicKey,
  // TODO(joe): possibly include other keys, pending zei interface updates.
  // eg. encryption key
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountAddress {
  pub key: XfrPublicKey,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedAddress {
  pub address: XfrAddress,
  pub signature: XfrSignature,
}

impl SignedAddress {
  pub fn verify(&self, message: &[u8]) -> bool {
    self.address.key.verify(message, &self.signature).is_ok()
  }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Asset {
  pub code: AssetTypeCode,
  pub issuer: IssuerPublicKey,
  pub memo: Memo,
  pub confidential_memo: ConfidentialMemo,
  pub updatable: bool,
  pub traceable: bool,
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub policy: Option<(Box<Policy>, PolicyGlobals)>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AssetType {
  pub properties: Asset,
  pub digest: [u8; 32],
  pub units: u64,
  pub confidential_units: Commitment,
}

//impl AssetType {
//    pub fn create_empty() -> AssetType {
//        AssetType {
//            code: AssetTypeCode{val:[0;16]},
//            digest: [0;32],
//            issuer: Address{key:[0;32]},
//            memo: Memo{},
//            confidential_memo: ConfidentialMemo{},
//            updatable: false,
//            units: 0,
//            confidential_units: [0;32],
//        }
//    }
//}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CustomAssetPolicy {
  policy: Vec<u8>, // serialized policy, underlying form TBD.
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CredentialProofKey([u8; 16]);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CredentialProof {
  pub key: CredentialProofKey,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SmartContract;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TxoSID(pub u64);

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TxnSID(pub usize);

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct BlockSID(pub usize);

// An ephemeral index for a transaction (with a different newtype so that
// it's harder to mix up)
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TxnTempSID(pub usize);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TxOutput(pub BlindAssetRecord);

#[derive(Eq, PartialEq, Debug)]
pub enum UtxoStatus {
  Spent,
  Unspent,
  Nonexistent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Utxo(pub TxOutput);
// TODO(joe): the digest is currently unused -- should it be put back?
// pub struct Utxo {
//   // digest is a hash of the TxoSID and the operation output
//   pub digest: [u8; 32],
//   pub output: TxOutput,
// }

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TxoRef {
  // Offset backwards from this operation (within a txn) -- 0 is the most recent, (n-1) (if there
  // are n outputs so far) is the first output of the transaction
  Relative(u64),
  // Absolute Txo address to a location outside this txn
  Absolute(TxoSID),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransferAssetBody {
  pub inputs: Vec<TxoRef>, // Ledger address of inputs
  pub num_outputs: usize,  // How many output TXOs?
  // TODO(joe): we probably don't need the whole XfrNote with input records
  // once it's on the chain
  pub transfer: Box<XfrBody>, // Encrypted transfer note
}

impl TransferAssetBody {
  pub fn new<R: CryptoRng + RngCore>(prng: &mut R,
                                     input_refs: Vec<TxoRef>,
                                     input_records: &[OpenAssetRecord],
                                     output_records: &[AssetRecord])
                                     -> Result<TransferAssetBody, errors::PlatformError> {
    if input_records.is_empty() {
      return Err(PlatformError::InputsError);
    }
    let in_records =
      input_records.iter()
                   .map(|oar| AssetRecord::from_open_asset_record_no_asset_tracking(oar.clone()))
                   .collect_vec();
    let note = Box::new(gen_xfr_body(prng, in_records.as_slice(), output_records)?);
    Ok(TransferAssetBody { inputs: input_refs,
                           num_outputs: output_records.len(),
                           transfer: note })
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IssueAssetBody {
  pub code: AssetTypeCode,
  pub seq_num: u64,
  pub num_outputs: usize,
  pub records: Vec<TxOutput>,
}

impl IssueAssetBody {
  pub fn new(token_code: &AssetTypeCode,
             seq_num: u64,
             records: &[TxOutput])
             -> Result<IssueAssetBody, PlatformError> {
    Ok(IssueAssetBody { code: *token_code,
                        seq_num,
                        num_outputs: records.len(),
                        records: records.to_vec() })
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DefineAssetBody {
  pub asset: Asset,
}

impl DefineAssetBody {
  pub fn new(token_code: &AssetTypeCode,
             issuer_key: &IssuerPublicKey, // TODO: require private key check somehow?
             updatable: bool,
             traceable: bool,
             memo: Option<Memo>,
             confidential_memo: Option<ConfidentialMemo>,
             policy: Option<(Box<Policy>, PolicyGlobals)>)
             -> Result<DefineAssetBody, PlatformError> {
    let mut asset_def: Asset = Default::default();
    asset_def.code = *token_code;
    asset_def.issuer = *issuer_key;
    asset_def.updatable = updatable;
    asset_def.traceable = traceable;
    asset_def.policy = policy;

    if let Some(memo) = memo {
      asset_def.memo = Memo(memo.0);
    } else {
      asset_def.memo = Memo(String::from(""));
    }

    if let Some(confidential_memo) = confidential_memo {
      asset_def.confidential_memo = confidential_memo;
    } else {
      asset_def.confidential_memo = ConfidentialMemo {};
    }
    Ok(DefineAssetBody { asset: asset_def })
  }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AIRAssignBody {
  pub addr: String,
  pub data: String,
}

impl AIRAssignBody {
  pub fn new(addr: String, data: String) -> Result<AIRAssignBody, errors::PlatformError> {
    Ok(AIRAssignBody { addr, data })
  }
}

pub fn compute_signature<T>(secret_key: &XfrSecretKey,
                            public_key: &XfrPublicKey,
                            operation_body: &T)
                            -> XfrSignature
  where T: serde::Serialize
{
  secret_key.sign(&serde_json::to_vec(&operation_body).unwrap(), &public_key)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransferType {
  Standard,
  DebtSwap,
}

impl Default for TransferType {
  fn default() -> Self {
    Self::Standard
  }
}

// TODO: UTXO Addresses must be included in Transfer Signature
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransferAsset {
  pub body: TransferAssetBody,
  pub transfer_type: TransferType,
  pub body_signatures: Vec<SignedAddress>,
}

impl TransferAsset {
  pub fn new(transfer_body: TransferAssetBody,
             transfer_type: TransferType)
             -> Result<TransferAsset, PlatformError> {
    Ok(TransferAsset { body: transfer_body,
                       body_signatures: Vec::new(),
                       transfer_type })
  }

  pub fn sign(&mut self, keypair: &XfrKeyPair) {
    let sig = keypair.get_sk_ref()
                     .sign(&serde_json::to_vec(&self.body).unwrap(),
                           keypair.get_pk_ref());

    self.body_signatures
        .push(SignedAddress { signature: sig,
                              address: XfrAddress { key: *keypair.get_pk_ref() } });
  }
}

// TODO: Include mechanism for replay attacks
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IssueAsset {
  pub body: IssueAssetBody,
  pub pubkey: IssuerPublicKey,
  pub signature: XfrSignature,
}

impl IssueAsset {
  pub fn new(issuance_body: IssueAssetBody,
             public_key: &IssuerPublicKey,
             secret_key: &XfrSecretKey)
             -> Result<IssueAsset, PlatformError> {
    let sign = compute_signature(&secret_key, &public_key.key, &issuance_body);
    Ok(IssueAsset { body: issuance_body,
                    pubkey: *public_key,
                    signature: sign })
  }
}

// ... etc...
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DefineAsset {
  pub body: DefineAssetBody,

  // TODO(joe?): Why is there a distinct public key used for signing?
  // Should this be the same as the issuer key in `body`? Is it *dangerous*
  // to have a distinct public key for this? Is it *beneficial* to have a
  // distinct public key?
  pub pubkey: IssuerPublicKey,
  pub signature: XfrSignature,
}

impl DefineAsset {
  pub fn new(creation_body: DefineAssetBody,
             public_key: &IssuerPublicKey,
             secret_key: &XfrSecretKey)
             -> Result<DefineAsset, PlatformError> {
    let sign = compute_signature(&secret_key, &public_key.key, &creation_body);
    Ok(DefineAsset { body: creation_body,
                     pubkey: *public_key,
                     signature: sign })
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AIRAssign {
  pub body: AIRAssignBody,
  pub pubkey: IssuerPublicKey,
  pub signature: XfrSignature,
}

impl AIRAssign {
  pub fn new(creation_body: AIRAssignBody,
             public_key: &IssuerPublicKey,
             secret_key: &XfrSecretKey)
             -> Result<AIRAssign, errors::PlatformError> {
    let sign = compute_signature(&secret_key, &public_key.key, &creation_body);
    Ok(AIRAssign { body: creation_body,
                   pubkey: *public_key,
                   signature: sign })
  }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Operation {
  TransferAsset(TransferAsset),
  IssueAsset(IssueAsset),
  DefineAsset(DefineAsset),
  AIRAssign(AIRAssign),
  // ... etc...
}

#[derive(Clone, Debug)]
pub struct TimeBounds {
  pub start: DateTime<Utc>,
  pub end: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Default)]
pub struct Transaction {
  pub operations: Vec<Operation>,
  pub credentials: Vec<CredentialProof>,
  #[serde(default)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub policy_options: Option<TxnPolicyData>,
  pub memos: Vec<Memo>,
  #[serde(default)]
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub signatures: Vec<XfrSignature>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalizedTransaction {
  pub txn: Transaction,
  pub tx_id: TxnSID,

  pub merkle_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct AuthenticatedTransaction {
  pub finalized_txn: FinalizedTransaction,
  pub txn_inclusion_proof: Proof,
  pub state_commitment_data: StateCommitmentData,
  pub state_commitment: BitDigest,
}

impl AuthenticatedTransaction {
  // An authenticated txn result is valid if
  // 1) The state commitment used in the proof matches what we pass in and the state commitment
  //    data hashes to the state commitment
  // 2) The transaction merkle proof is valid
  // 3) The transaction merkle root matches the value in root_hash_data
  pub fn is_valid(&self, state_commitment: BitDigest) -> bool {
    //1)
    if self.state_commitment != state_commitment
       || self.state_commitment != self.state_commitment_data.compute_commitment()
    {
      return false;
    }

    //2)
    let hash = self.finalized_txn.hash();

    if !self.txn_inclusion_proof.is_valid_proof(hash) {
      return false;
    }

    //3)
    // TODO (jonathan/noah) we should be using digest everywhere
    if self.state_commitment_data.transaction_merkle_commitment
       != self.txn_inclusion_proof.root_hash
    {
      return false;
    }

    true
  }
}

pub struct AuthenticatedBlock {
  pub block: FinalizedBlock,
  pub block_inclusion_proof: Proof,
  pub state_commitment_data: StateCommitmentData,
  pub state_commitment: BitDigest,
}

impl AuthenticatedBlock {
  // An authenticated block result is valid if
  // 1) The block merkle proof is valid
  // 2) The block merkle root matches the value in root_hash_data
  // 3) root_hash_data hashes to root_hash
  // 4) The state commitment of the proof matches the state commitment passed in
  pub fn is_valid(&self, state_commitment: BitDigest) -> bool {
    //1) compute block hash
    let txns: Vec<Transaction> = self.block
                                     .txns
                                     .iter()
                                     .map(|auth_tx| auth_tx.txn.clone())
                                     .collect();
    let serialized = bincode::serialize(&txns).unwrap();
    let digest = sha256::hash(&serialized);
    let mut hash = HashValue::new();
    hash.hash.clone_from_slice(&digest.0);

    if self.block_inclusion_proof.is_valid_proof(hash) {
      return false;
    }

    //2)
    if self.state_commitment_data.block_merkle != self.block_inclusion_proof.root_hash {
      return false;
    }

    //3) 4)
    if self.state_commitment != self.state_commitment_data.compute_commitment()
       || state_commitment != self.state_commitment
    {
      return false;
    }

    true
  }
}

pub struct AuthenticatedUtxoStatus {
  pub status: UtxoStatus,
  pub utxo_sid: TxoSID,
  pub state_commitment_data: StateCommitmentData,
  pub utxo_map: Option<SparseMap>, // BitMap only needed for proof if the txo_sid exists
  pub state_commitment: BitDigest,
}

impl AuthenticatedUtxoStatus {
  // An authenticated utxo status is valid (for txos that exist) if
  // 1) The state commitment of the proof matches the state commitment passed in
  // 2) The state commitment data hashes to the state commitment
  // 3) The status matches the bit stored in the bitmap
  // 4) The bitmap checksum matches digest in state commitment data
  // 5) For txos that don't exist, simply show that the utxo_sid greater than max_sid
  pub fn is_valid(&self, state_commitment: BitDigest) -> bool {
    let state_commitment_data = &self.state_commitment_data;
    let utxo_sid = self.utxo_sid.0;
    // 1, 2) First, validate the state commitment
    if state_commitment != self.state_commitment
       || self.state_commitment != state_commitment_data.compute_commitment()
    {
      return false;
    }
    // If the txo exists, the proof must also contain a bitmap
    let utxo_map = self.utxo_map.as_ref().unwrap();
    // 3) The status matches the bit stored in the bitmap
    let spent = !utxo_map.query(utxo_sid).unwrap();
    if (self.status == UtxoStatus::Spent && !spent) || (self.status == UtxoStatus::Unspent && spent)
    {
      return false;
    }
    // 4)
    if utxo_map.checksum() != self.state_commitment_data.bitmap {
      println!("failed at bitmap checksum");
      return false;
    }

    if self.status == UtxoStatus::Nonexistent {
      // 5)
      return utxo_sid >= state_commitment_data.txo_count;
    }

    true
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalizedBlock {
  pub txns: Vec<FinalizedTransaction>,
  pub merkle_id: u64,
}

impl FinalizedTransaction {
  pub fn hash(&self) -> HashValue {
    self.txn.hash(self.tx_id)
  }
}

impl Transaction {
  pub fn add_operation(&mut self, op: Operation) {
    self.operations.push(op);
  }

  pub fn serialize_bincode(&self, sid: TxnSID) -> Vec<u8> {
    let mut serialized = bincode::serialize(&self).unwrap();
    serialized.extend(bincode::serialize(&sid).unwrap());
    serialized
  }

  pub fn hash(&self, sid: TxnSID) -> HashValue {
    let digest = sha256::hash(&self.serialize_bincode(sid));
    let mut hash = HashValue::new();
    hash.hash.clone_from_slice(&digest.0);
    hash
  }

  fn serialize_without_sigs(&self) -> Vec<u8> {
    // TODO(joe): do this without a clone?
    let mut other_txn;
    let base_txn = if self.signatures.is_empty() {
      &self
    } else {
      other_txn = self.clone();
      other_txn.signatures.clear();
      &other_txn
    };
    serde_json::to_vec(base_txn).unwrap()
  }

  pub fn sign(&mut self, secret_key: &XfrSecretKey, public_key: &XfrPublicKey) {
    let sig = secret_key.sign(&self.serialize_without_sigs(), &public_key);
    self.signatures.push(sig);
  }

  pub fn check_signature(&self,
                         public_key: &XfrPublicKey,
                         sig: &XfrSignature)
                         -> Result<(), PlatformError> {
    public_key.verify(&self.serialize_without_sigs(), sig)?;
    Ok(())
  }

  /// NOTE: this does *not* guarantee that a private key affiliated with
  /// `public_key` has signed this transaction! If `public_key` is derived
  /// from `self` somehow, then it is infeasible for someone to forge a
  /// passing signature, but it is plausible for someone to generate an
  /// unrelated `public_key` which can pass this signature check!
  pub fn check_has_signature(&self, public_key: &XfrPublicKey) -> Result<(), PlatformError> {
    let serialized = self.serialize_without_sigs();
    for sig in self.signatures.iter() {
      match public_key.verify(&serialized, sig) {
        Err(_) => {}
        Ok(_) => {
          return Ok(());
        }
      }
    }
    Err(PlatformError::InputsError)
  }
}

#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// TODO (Keyao):
// Are the four fields below all necessary?
// Can we remove one of txns_in_block_hash and global_block_hash?
// Both of them contain the information of the previous state
pub struct StateCommitmentData {
  pub bitmap: BitDigest,                        // The checksum of the utxo_map
  pub block_merkle: HashValue,                  // The root hash of the block Merkle tree
  pub txns_in_block_hash: BitDigest,            // The hash of the transactions in the block
  pub previous_state_commitment: BitDigest,     // The prior global block hash
  pub transaction_merkle_commitment: HashValue, // The root hash of the transaction Merkle tree
  pub air_commitment: BitDigest,                // The root hash of the AIR sparse Merkle tree
  pub txo_count: u64, // Number of transaction outputs. Used to provide proof that a utxo does not exist
}

impl StateCommitmentData {
  pub fn compute_commitment(&self) -> BitDigest {
    let serialized = serde_json::to_string(&self).unwrap();
    sha256::hash(&serialized.as_bytes())
  }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountID {
  pub val: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Account {
  pub id: AccountID,
  pub access_control_list: Vec<AccountAddress>,
  pub key_value: HashMap<String, String>, //key value storage...
}

#[cfg(test)]
mod tests {
  use super::*;
  use rand_core::SeedableRng;
  use std::cmp::min;
  use zei::xfr::structs::{AssetTypeAndAmountProof, XfrBody, XfrProofs};

  #[test]
  fn test_gen_random() {
    let mut sum: u64 = 0;
    let mut sample_size = 0;

    for _ in 0..1000 {
      let code = AssetTypeCode::gen_random();
      let mut failed = true;

      for byte in code.val.iter() {
        if *byte != 0 {
          failed = false;
        }

        sum += *byte as u64;
        sample_size += 1;
      }

      assert!(!failed);
    }

    // Use the central limit theorem.  The standard deviation of the
    // sample mean should be normal(127.5, uniform variance).  Work
    // from the standard deviation of uniform(0, 1), sqrt(1/12).  The
    // expected average (mu) is 127.5 if the random number generator
    // is unbiased.
    let uniform_stddev = 1.0 / (12.0 as f64).sqrt();
    let average = sum as f64 / sample_size as f64;
    let stddev = (uniform_stddev * 255.0) / (sample_size as f64).sqrt();
    println!("Average {}, stddev {}", average, stddev);
    assert!(average > 127.5 - 3.0 * stddev);
    assert!(average < 127.5 + 3.0 * stddev);
  }

  #[test]
  fn test_new_from_str() {
    let value = "1";
    let mut input = "".to_string();

    for i in 0..64 {
      let code = AssetTypeCode::new_from_str(&input);
      let mut checked = 0;

      for j in 0..min(i, code.val.len()) {
        assert!(code.val[j] == value.as_bytes()[0]);
        checked = checked + 1;
      }

      for j in i..code.val.len() {
        assert!(code.val[j] == 0);
        checked = checked + 1;
      }

      assert!(checked == code.val.len());
      input = input + &value;
    }
  }

  #[test]
  fn test_new_from_base64() {
    let base64 = "ZGVmZ2hpamtsbW5vcHFycw==";
    let result = Code::new_from_base64(base64);

    assert_eq!(result.ok(),
               Some(Code { val: [100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
                                 112, 113, 114, 115] }));
  }

  #[test]
  fn test_code_to_base64() {
    let code = Code { val: [100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
                            113, 114, 115] };
    assert_eq!(code.to_base64(), "ZGVmZ2hpamtsbW5vcHFycw==");
  }

  #[test]
  fn test_verify() {
    let mut prng = rand_chacha::ChaChaRng::from_entropy();

    let keypair = XfrKeyPair::generate(&mut prng);
    let message: &[u8] = b"test";

    let signed_address = SignedAddress { address: XfrAddress { key: *keypair.get_pk_ref() },
                                         signature: keypair.sign(message) };

    assert!(signed_address.verify(message));
  }

  // Test Transaction::add_operation
  // Below are not directly tested but called:
  //   TransferAssetBody::new
  //   IssueAssetBody::new
  //   DefineAssetBody::new
  //   TransferAsset::new
  //   IssueAsset::new
  //   DefineAsset::new
  #[test]
  fn test_add_operation() {
    // Create values to be used to instantiate operations
    let mut transaction: Transaction = Default::default();

    let mut prng = rand_chacha::ChaChaRng::from_entropy();

    let keypair = XfrKeyPair::generate(&mut prng);
    let message: &[u8] = b"test";

    let public_key = *keypair.get_pk_ref();
    let signature = keypair.sign(message);

    // Instantiate an TransferAsset operation
    let xfr_note = XfrBody { inputs: Vec::new(),
                             outputs: Vec::new(),
                             proofs: XfrProofs { asset_type_and_amount_proof:
                                                   AssetTypeAndAmountProof::NoProof,
                                                 asset_tracking_proof: Default::default() },
                             asset_tracing_memos: vec![],
                             owners_memos: vec![] };

    let assert_transfer_body = TransferAssetBody { inputs: Vec::new(),
                                                   num_outputs: 0,
                                                   transfer: Box::new(xfr_note) };

    let asset_transfer = TransferAsset { body: assert_transfer_body,
                                         body_signatures: Vec::new(),
                                         transfer_type: TransferType::Standard };

    let transfer_operation = Operation::TransferAsset(asset_transfer.clone());

    // Instantiate an IssueAsset operation
    let asset_issuance_body = IssueAssetBody { code: Default::default(),
                                               seq_num: 0,
                                               num_outputs: 0,
                                               records: Vec::new() };

    let asset_issuance = IssueAsset { body: asset_issuance_body,
                                      pubkey: IssuerPublicKey { key: public_key },
                                      signature: signature.clone() };

    let issuance_operation = Operation::IssueAsset(asset_issuance.clone());

    // Instantiate an DefineAsset operation
    let asset = Default::default();

    let asset_creation = DefineAsset { body: DefineAssetBody { asset },
                                       pubkey: IssuerPublicKey { key: public_key },
                                       signature: signature.clone() };

    let creation_operation = Operation::DefineAsset(asset_creation.clone());

    // Instantiate an AIRAssign operation
    let air_assign_body = AIRAssignBody { addr: String::from(""),
                                          data: String::from("") };

    let air_assign = AIRAssign { body: air_assign_body,
                                 pubkey: IssuerPublicKey { key: public_key },
                                 signature: signature.clone() };

    let air_assign_operation = Operation::AIRAssign(air_assign.clone());

    // Add operations to the transaction
    transaction.add_operation(transfer_operation);
    transaction.add_operation(issuance_operation);
    transaction.add_operation(creation_operation);
    transaction.add_operation(air_assign_operation);

    // Verify operatoins
    assert_eq!(transaction.operations.len(), 4);

    assert_eq!(transaction.operations.get(0),
               Some(&Operation::TransferAsset(asset_transfer)));
    assert_eq!(transaction.operations.get(1),
               Some(&Operation::IssueAsset(asset_issuance)));
    assert_eq!(transaction.operations.get(2),
               Some(&Operation::DefineAsset(asset_creation)));
    assert_eq!(transaction.operations.get(3),
               Some(&Operation::AIRAssign(air_assign)));
  }

  // Verify that the hash values of two transactions:
  //   are the same if the transactions differ only in merkle_id
  //   are different if the transactions differ in other fields
  // TODO(joe): determine a good test to replace this
  // #[test]
  // fn test_compute_merkle_hash() {
  //   let transaction_default: Transaction = Default::default();

  //   let transaction_different_merkle_id =
  //     Transaction { operations: Vec::new(),
  //                   credentials: Vec::new(),
  //                   memos: Vec::new() };

  //   let transaction_other_differences = Transaction { operations: Vec::new(),
  //                                                     credentials: Vec::new(),
  //                                                     memos: Vec::new(),
  //                                                     };

  //   let hash_value_default = transaction_default.compute_merkle_hash();
  //   let hash_value_different_merkle_id = transaction_different_merkle_id.compute_merkle_hash();
  //   let hash_value_other_differences = transaction_other_differences.compute_merkle_hash();

  //   assert_eq!(hash_value_different_merkle_id, hash_value_default);
  //   assert_ne!(hash_value_other_differences, hash_value_default);
  // }
}
