use crate::{ProofOfWork, Transaction};

use serde::{Deserialize, Serialize};
use sled::IVec;

#[derive(Clone, Serialize, Deserialize)]
pub struct Block {
    timestamp: i64,
    prev_block_hash: String,
    hash: String,
    transactions: Vec<Transaction>,
    nonce: i64,
    height: usize,
}

impl Block {
    pub fn new_block(
        prev_block_hash: String,
        transactions: &[Transaction],
        height: usize,
    ) -> Block {
        let mut block = Block {
            timestamp: crate::current_timestamp(),
            prev_block_hash,
            hash: String::new(),
            transactions: transactions.to_vec(),
            nonce: 0,
            height,
        };

        let pow = ProofOfWork::new_proof_of_work(block.clone());
        let (nonce, hash) = pow.run();
        block.hash = hash;
        block.nonce = nonce;
        block
    }

    pub fn deserialize(bytes: &[u8]) -> Block {
        bincode::deserialize(bytes).unwrap()
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap().to_vec()
    }

    pub fn generate_genesis_block(transaction: &Transaction) -> Block {
        let transactions = vec![transaction.clone()];

        Block::new_block(String::from("none"), &transactions, 0)
    }

    pub fn hash_transactions(&self) -> Vec<u8> {
        let mut txhashes = vec![];
        for tx in &self.transactions {
            txhashes.extend(tx.get_id());
        }
        crate::sha256_digest(txhashes.as_slice())
    }

    pub fn get_transactions(&self) -> &[Transaction] {
        self.transactions.as_slice()
    }

    pub fn get_prev_block_hash(&self) -> String {
        self.prev_block_hash.clone()
    }

    pub fn get_hash(&self) -> &str {
        self.hash.as_str()
    }

    pub fn get_hash_bytes(&self) -> Vec<u8> {
        self.hash.as_bytes().to_vec()
    }

    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn get_height(&self) -> usize {
        self.height
    }
}

impl From<Block> for IVec {
    fn from(value: Block) -> Self {
        let bytes = bincode::serialize(&value).unwrap();
        Self::from(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::Block;
    use crate::Transaction;

    #[test]
    fn test_new_block() {
        let block = Block::new_block(
            String::from("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"),
            &vec![],
            0,
        );
        println!("new block hash is {}", block.hash)
    }

    #[test]
    fn test_block_serialize() {
        let tx = Transaction::new_coinbase_tx("Genesis");
        let block = Block::new_block(
            String::from("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"),
            &vec![tx],
            0,
        );
        let block_bytes = block.serialize();
        let desc_block = Block::deserialize(&block_bytes[..]);
        assert_eq!(block.hash, desc_block.hash)
    }
}
