# Wallet balance for Anonymous Tokens

## Context

### UTXO
The wallet presently shows the balance of an address .It uses
```rust
    pub fn get_owned_utxos(&self, addr: &XfrPublicKey) -> Vec<TxoSID> {
        self.utxos
            .iter()
            .filter(|(_, utxo)| &utxo.0.record.public_key == addr)
            .map(|(sid, _)| *sid)
            .collect()
    }
```
To get a list of TxoID , and then 
```rust
fn get_utxo(&self, addr: TxoSID) -> Option<AuthenticatedUtxo> {
    let utxo = self.status.get_utxo(addr);
    if let Some(utxo) = utxo.cloned() {
        let txn_location = *self.status.txo_to_txn_location.get(&addr).unwrap();
        let authenticated_txn = self.get_transaction(txn_location.0).unwrap();
        let authenticated_spent_status = self.get_utxo_status(addr);
        let state_commitment_data =
            self.status.state_commitment_data.as_ref().unwrap().clone();
        let utxo_location = txn_location.1;
        Some(AuthenticatedUtxo {
            utxo,
            authenticated_txn,
            authenticated_spent_status,
            utxo_location,
            state_commitment_data,
        })
    } else {
        None
    }
}
```
To get the utxos . The balance is the sum for all such utxos.

### Anonymous UTXO
- We cannot use a similar flow , becuase the the ledger only stores Anon records 
```rust
pub struct AnonBlindAssetRecord {
    pub amount_type_commitment: Commitment,
    pub public_key: AXfrPubKey,
}
```
Which contains commitments and not the actual amount .

## Proposal for balances
- A user can fetch the list of records tagged to their diversified public key (Fns or Wallet)
```rust
    pub fn get_owned_abar_records(&self, addr: &AXfrPubKey) -> Vec<AnonBlindAssetRecord> {
        self.ax_utxos
            .iter()
            .filter(|(_, axutxo)| &axutxo.public_key == addr)
            .map(|(_, record)| record.clone())
            .collect()
    }
```
- The user can fetch the memo using (Fns or Wallet)
```rust
fn get_abar_memo(&self,ax_id :ATxoSID) -> Option<Vec<Memo>>{
    let txn_location = *self.status.ax_txo_to_txn_location.get(&ax_id).unwrap();
    let authenticated_txn = self.get_transaction(txn_location.0).unwrap();
    let memo = authenticated_txn.finalized_txn.txn.body.memos;
    if memo.is_empty(){
        return None;
    }
    Some(memo)
}
}
```

- The user can then open these records , using  (Fns or Wallet)

```rust
    pub fn from_abar(
        record: &AnonBlindAssetRecord,
        owner_memo: OwnerMemo,
        key_pair: &AXfrKeyPair,
        dec_key: &XSecretKey,
    ) -> Result<Self> {
        let (amount, asset_type, blind, key_rand) =
            decrypt_memo(&owner_memo, dec_key, key_pair, record).c(d!())?;
        let mut builder = OpenAnonBlindAssetRecordBuilder::new()
            .pub_key(key_pair.pub_key())
            .amount(amount)
            .asset_type(asset_type);

        builder.oabar.blind = blind;
        builder.oabar.key_rand_factor = key_rand;
        builder.oabar.owner_memo = Some(owner_memo);
        Ok(builder)
    }
```
- The OpenAnonBlindAssetRecord contains the amount , which can be used to show wallet balance 
