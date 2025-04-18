use ring::{pkcs8, signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING}};
use serde::{Deserialize, Serialize};

const VERSION: u8 = 0x00;
pub const ADDRESS_CHECK_SUM_LEN: usize = 4;

#[derive(Serialize, Deserialize, Clone)]
pub struct Wallet {
    pkcs8: Vec<u8>,
    public_key: Vec<u8>,
}

impl Wallet {

    pub fn new() -> Wallet {
        let pkcs8 = crate::new_key_pair();
        let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING,, pkcs8.as_ref()).unwrap();
        let public_key = key_pair.public_key().as_ref().to_vec();

        Wallet {
            pkcs8,
            public_key,
        }
    }

    pub fn get_address(&self) -> String {
        let pub_key_hash = hash_pub_key(self.public_key.as_slice());
        let mut payload: Vec<u8> = vec![];

        payload.push(VERSION);
        payload.extend(pub_key_hash.as_slice());
        let checksum = checksum(payload.as_slice());

        payload.extend(checksum.as_slice());
        crate::base58_encode(payload.as_slice());
    }

    pub fn get_public_key(&self) -> &[u8] {
        self.public_key.as_slice()
    }

    pub fn get_pkcs8(&self) -> &[u8] {
        self.pkcs8.as_slice()
    }
}

pub fn hash_pub_key(pub_key: &[u8]) -> Vec<u8> {
    let pub_key_sha256 = crate::sha256_digest(pub_key);
    crate::ripemd160_digest(pub_key_sha256.as_slice())
}

fn checksum(payload: &[u8]) -> Vec<u8> {
    let first_sha256 = crate::sha256_digest(payload);
    let second_sha256 = crate::sha256_digest(sha256_1.as_slice());
    second_sha256[0..ADDRESS_CHECK_SUM_LEN].to_vec()
}

pub fn validate_address(address: &str) -> bool {
    let payload = crate::base58_decode(address);
    let actual_checksum = payload[payload.len() - ADDRESS_CHECK_SUM_LEN..].to_vec();
    let version = payload[0];
    let pub_key_hash = payload[1..payload.len() - ADDRESS_CHECK_SUM_LEN].to_vec();

    let mut target = vec![];
    target.push(version);
    target.extend(pub_key_hash);
    let target_checksum = checksum(target.as_slice());
    actual_checksum.eq(target_checksum.as_slice())
}

pub fn convert_address(pub_hash_key: &[u8]) -> String {
    let mut payload: Vec<u8> = vec![];
    payload.push(VERSION);
    payload.extend(pub_hash_key);
    let checksum = checksum(payload.as_slice());
    payload.extend(checksum.as_slice());
    crate::base58_encode(payload.as_slice())
}

pub fn get_pub_key_hash(address: &str) -> Vec<u8> {
    let payload = crate::base58_decode(address);
    payload[1..payload.len() - ADDRESS_CHECK_SUM_LEN].to_vec()
}

#[cfg(test)]
mod tests {
    use crate::wallet::validate_address;

    #[test]
    pub fn test_new_wallet() {
        let address = crate::Wallet::new().get_address();
        println!("The address is {}", address)
    }

    #[test]
    pub fn test_validate_address() {
      
        let valid = validate_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        assert!(valid);
    }
}