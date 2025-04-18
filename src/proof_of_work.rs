use crate::Block;

use std::borrow::Borrow;
use std::ops::ShlAssign;

use data_encoding::HEXLOWER;
use num_bigint::{BigInt, Sign};

pub struct ProofOfWork {
    block: Block,
    target: BigInt,
}

const TARGET_BITS: i32 = 8;
const MAX_NONCE: i64 = i64::MAX;

impl ProofOfWork {
    pub fn new_proof_of_work(block: Block) -> ProofOfWork {
        let mut target = BigInt::from(1);

        target.shl_assign(256 - TARGET_BITS);
        ProofOfWork { block, target }
    }

    fn prepare_data(&self, nonce: i64) -> Vec<u8> {
        let prev_block_hash = self.block.get_prev_block_hash();
        let transaction_hash = self.block.hash_transactions();
        let timestamp = self.block.get_timestamp();

        let mut data_bytes = vec![];

        data_bytes.extend(prev_block_hash.as_bytes());
        data_bytes.extend(transaction_hash);
        data_bytes.extend(timestamp.to_be_bytes());
        data_bytes.extend(TARGET_BITS.to_be_bytes());
        data_bytes.extend(nonce.to_be_bytes());

        data_bytes
    }

    pub fn run(&self) -> (i64, String) {
        let mut nonce = 0;
        let mut hash = Vec::new();
        println!("Mining the block...");

        while nonce < MAX_NONCE {
            let data = self.prepare_data(nonce);
            hash = crate::sha256_digest(data.as_slice());
            let hash_int = BigInt::from_bytes_be(Sign::Plus, hash.as_slice());

            if hash_int.lt(self.target.borrow()) {
                println!("Hash: {}", HEXLOWER.encode(hash.as_slice()));
                println!("Nonce: {}", nonce);
                break;
            } else {
                nonce += 1;
            }
        }
        println!();
        (nonce, HEXLOWER.encode(hash.as_slice()))
    }
}

#[cfg(test)]
mod test {
    use super::TARGET_BITS;
    use data_encoding::HEXLOWER;
    use num_bigint::BigInt;
    use std::ops::ShlAssign;

    #[test]
    fn test_target_bits() {
        let mut target = BigInt::from(1);
        target.shl_assign(256 - TARGET_BITS);

        println!("Target: {}", target);

        let (_, vec) = target.to_bytes_be();
        let target_hex = HEXLOWER.encode(vec.as_slice());
        println!("Target hex: {}", target_hex);
    }

    #[test]
    fn test_bigint_from_bytes() {
        let a = BigInt::from(256);
        let (s, vec) = a.to_bytes_be();
        println!("{:?}, {:?}", s, vec);

        let b = BigInt::from_signed_bytes_be(vec.as_slice());
        println!("{:?}", b);
    }
}
