use std::iter::repeat;
use std::time::{SystemTime, UNIX_EPOCH};

use crypto::digest::Digest;
use ring::digest::{Context, SHA256};
use ring::rand::SystemRandom;
use ring::signature::{ECDSA_P256_SHA256_FIXED, ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair};

pub fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as i64
}

pub fn sha256_digest(data: &[u8]) -> Vec<u8> {
    let mut context = Context::new(&SHA256);
    context.update(data);
    context.finish().as_ref().to_vec()
}

pub fn ripemd160_digest(data: &[u8]) -> Vec<u8> {
    let mut ripemd160 = crypto::ripemd160::Ripemd160::new();
    ripemd160.input(data);
    let mut buf: Vec<u8> = repeat(0).take(ripemd160.output_bytes()).collect();
    ripemd160.result(&mut buf);
    buf
}

pub fn base58_encode(data: &[u8]) -> String {
    bs58::encode(data).into_string()
}

pub fn base58_decode(data: &str) -> Vec<u8> {
    bs58::decode(data).into_vec().unwrap()
}

pub fn new_key_pair() -> Vec<u8> {
    let rng = SystemRandom::new();

    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    pkcs8.as_ref().to_vec()
}

pub fn ecdsa_p256_sha256_sign_digest(pkcs8: &[u8], message: &[u8]) -> Vec<u8> {
    let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8).unwrap();
    let rng = SystemRandom::new();

    key_pair.sign(&rng, message).unwrap().as_ref().to_vec()
}

pub fn ecdsa_p256_sha256_sign_verify(public_key: &[u8], signature: &[u8], message: &[u8]) -> bool {
    let peer_public_key =
        ring::signature::UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, public_key);

    let result = peer_public_key.verify(message, signature.as_ref());
    result.is_ok()
}

#[cfg(test)]
mod tests {
    use crate::new_key_pair;
    use data_encoding::HEXLOWER;
    use ring::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};

    #[test]
    fn test_sha256_digest() {
        let digest = crate::sha256_digest("hello".as_bytes());

        let hex_digest = HEXLOWER.encode(digest.as_slice());
        println!("SHA-256 digest is {}", hex_digest)
    }

    #[test]
    fn test_ripemd160() {
        let bytes = crate::ripemd160_digest("mars".as_bytes());
        let hex_str = HEXLOWER.encode(bytes.as_slice());

        println!("ripemd160 digest is {}", hex_str)
    }

    #[test]
    fn test_base58() {
        let sign = "dd2324928f0552d4f4c6e57d9e5f6009ab085d85";
        let base58_sign = crate::base58_encode(sign.as_bytes());

        let decode_bytes = crate::base58_decode(base58_sign.as_str());
        let decode_str = String::from_utf8(decode_bytes).unwrap();
        assert_eq!(sign, decode_str.as_str());
    }

    #[test]
    fn test_ecdsa_sign_and_verify() {
        const MESSAGE: &[u8] = b"hello, world";
        let pkcs8 = new_key_pair();

        let signature = crate::ecdsa_p256_sha256_sign_digest(pkcs8.as_slice(), MESSAGE);

        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_slice()).unwrap();
        let public_key = key_pair.public_key().as_ref();
        let verify =
            crate::ecdsa_p256_sha256_sign_verify(public_key, signature.as_slice(), MESSAGE);
        assert!(verify)
    }
}
