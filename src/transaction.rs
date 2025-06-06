use crate::wallet::hash_pub_key;
use crate::{Blockchain, UTXOSet, Wallets, base58_decode, wallet};

use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SUBSIDY: i32 = 10;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct TXInput {
    txid: Vec<u8>,
    vout: usize,
    signature: Vec<u8>,
    pub_key: Vec<u8>,
}

impl TXInput {
    pub fn new(txid: &[u8], vout: usize) -> TXInput {
        TXInput {
            txid: txid.to_vec(),
            vout,
            signature: vec![],
            pub_key: vec![],
        }
    }

    pub fn get_txid(&self) -> &[u8] {
        self.txid.as_slice()
    }

    pub fn get_vout(&self) -> usize {
        self.vout
    }

    pub fn get_pub_key(&self) -> &[u8] {
        self.pub_key.as_slice()
    }

    pub fn uses_key(&self, pub_key_hash: &[u8]) -> bool {
        let locking_hash = wallet::hash_pub_key(self.pub_key.as_slice());

        locking_hash.eq(pub_key_hash)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TXOutput {
    value: i32,
    pub_key_hash: Vec<u8>,
}

impl TXOutput {
    pub fn new(value: i32, address: &str) -> TXOutput {
        let mut output = TXOutput {
            value,
            pub_key_hash: vec![],
        };
        output.lock(address);
        output
    }

    pub fn get_value(&self) -> i32 {
        self.value
    }

    pub fn get_pub_key_hash(&self) -> &[u8] {
        self.pub_key_hash.as_slice()
    }

    fn lock(&mut self, address: &str) {
        let payload = base58_decode(address);
        let pub_key_hash = payload[1..payload.len() - wallet::ADDRESS_CHECK_SUM_LEN].to_vec();
        self.pub_key_hash = pub_key_hash;
    }

    pub fn is_locked_with_key(&self, pub_key_hash: &[u8]) -> bool {
        self.pub_key_hash.eq(pub_key_hash)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Transaction {
    id: Vec<u8>,
    vin: Vec<TXInput>,
    vout: Vec<TXOutput>,
}

impl Transaction {
    pub fn new_coinbase_tx(to: &str) -> Transaction {
        let txout = TXOutput::new(SUBSIDY, to);
        let mut tx_input = TXInput::default();

        tx_input.signature = Uuid::new_v4().as_bytes().to_vec();

        let mut tx = Transaction {
            id: vec![],
            vin: vec![tx_input],
            vout: vec![txout],
        };

        tx.id = tx.hash();
        tx
    }

    pub fn new_utxo_transaction(
        from: &str,
        to: &str,
        amount: i32,
        utxo_set: &UTXOSet,
    ) -> Transaction {
        let wallets = Wallets::new();
        let wallet = wallets.get_wallet(from).expect("Wallet not found");

        let public_key_hash = hash_pub_key(wallet.get_public_key());

        let (accumlated, valid_outputs) =
            utxo_set.find_spendable_outputs(public_key_hash.as_slice(), amount);
        if accumlated < amount {
            panic!("Not enough funds")
        }

        let mut inputs = vec![];

        for (txid_hex, outs) in valid_outputs {
            let txid = HEXLOWER.decode(txid_hex.as_bytes()).unwrap();
            for out in outs {
                let input = TXInput {
                    txid: txid.clone(),
                    vout: out,
                    signature: vec![],
                    pub_key: wallet.get_public_key().to_vec(),
                };
                inputs.push(input);
            }
        }

        let mut outputs = vec![TXOutput::new(amount, to)];

        if accumlated > amount {
            outputs.push(TXOutput::new(accumlated - amount, from));
        }

        let mut tx = Transaction {
            id: vec![],
            vin: inputs,
            vout: outputs,
        };

        tx.id = tx.hash();
        tx.sign(utxo_set.get_blockchain(), wallet.get_pkcs8());
        tx
    }

    fn trimmed_copy(&self) -> Transaction {
        let mut inputs = vec![];
        let mut outputs = vec![];
        for input in &self.vin {
            let txinput = TXInput::new(input.get_txid(), input.get_vout());
            inputs.push(txinput);
        }
        for output in &self.vout {
            outputs.push(output.clone());
        }
        Transaction {
            id: self.id.clone(),
            vin: inputs,
            vout: outputs,
        }
    }

    fn sign(&mut self, blockchain: &Blockchain, pkcs8: &[u8]) {
        let mut tx_copy = self.trimmed_copy();

        for (idx, vin) in self.vin.iter_mut().enumerate() {
            let prev_tx_option = blockchain.find_transaction(vin.get_txid());
            if prev_tx_option.is_none() {
                panic!("Error: Previous transaction is not correct")
            }
            let prev_tx = prev_tx_option.unwrap();
            tx_copy.vin[idx].signature = vec![];
            tx_copy.vin[idx].pub_key = prev_tx.vout[vin.vout].pub_key_hash.clone();
            tx_copy.id = tx_copy.hash();
            tx_copy.vin[idx].pub_key = vec![];

            let signature = crate::ecdsa_p256_sha256_sign_digest(pkcs8, tx_copy.get_id());
            vin.signature = signature
        }
    }

    pub fn verify(&self, blockchain: &Blockchain) -> bool {
        if self.is_coinbase() {
            return true;
        }

        let mut tx_copy = self.trimmed_copy();

        for (idx, vin) in self.vin.iter().enumerate() {
            let prev_tx_option = blockchain.find_transaction(vin.get_txid());
            if prev_tx_option.is_none() {
                panic!("Error: Previous transaction is not correct")
            }
            let prev_tx = prev_tx_option.unwrap();
            tx_copy.vin[idx].signature = vec![];
            tx_copy.vin[idx].pub_key = prev_tx.vout[vin.vout].pub_key_hash.clone();
            tx_copy.id = tx_copy.hash();
            tx_copy.vin[idx].pub_key = vec![];

            let verify = crate::ecdsa_p256_sha256_sign_verify(
                vin.pub_key.as_slice(),
                vin.signature.as_slice(),
                tx_copy.get_id(),
            );

            if !verify {
                return false;
            }
        }
        true
    }

    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].pub_key.len() == 0
    }

    fn hash(&mut self) -> Vec<u8> {
        let tx_copy = Transaction {
            id: vec![],
            vin: self.vin.clone(),
            vout: self.vout.clone(),
        };
        crate::sha256_digest(tx_copy.serialize().as_slice())
    }

    pub fn get_id(&self) -> &[u8] {
        self.id.as_slice()
    }

    pub fn get_id_bytes(&self) -> Vec<u8> {
        self.id.clone()
    }

    pub fn get_vin(&self) -> &[TXInput] {
        self.vin.as_slice()
    }

    pub fn get_vout(&self) -> &[TXOutput] {
        self.vout.as_slice()
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap().to_vec()
    }

    pub fn deserialize(bytes: &[u8]) -> Transaction {
        bincode::deserialize(bytes).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Blockchain, Transaction, UTXOSet};
    use data_encoding::HEXLOWER;

    #[test]
    fn new_coinbase_tx() {
        let tx = Transaction::new_coinbase_tx("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        let txid_hex = HEXLOWER.encode(tx.get_id());
        println!("txid = {}", txid_hex);
    }

    #[test]
    fn test_blockchain_serialize() {
        let tx = Transaction::new_coinbase_tx("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        let tx_bytes = tx.serialize();
        let new_tx = Transaction::deserialize(tx_bytes.as_ref());
        assert_eq!(tx.get_id(), new_tx.get_id())
    }

    #[test]
    fn new_utxo_transaction() {
        let blockchain = Blockchain::new_blockchain();
        let utxo_set = UTXOSet::new(blockchain);
        let tx = Transaction::new_utxo_transaction(
            "13SDifQUyLGCwFjh64vihoWQcGsTozHuQb",
            "1LecNaLYsDoxRtxBBWKMNbLvccftmFZWcv",
            5,
            &utxo_set,
        );
        let txid_hex = HEXLOWER.encode(tx.get_id());

        println!("txid_hex = {}", txid_hex);
    }
}
