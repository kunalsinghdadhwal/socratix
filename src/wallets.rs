use crate::Wallet;

use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::vec;

pub const WALLET_FILE: &str = "wallets.dat";

pub struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Wallets {
    pub fn new() -> Wallets {
        let mut wallets = Wallets {
            wallets: HashMap::new(),
        };

        wallets.load_from_file();
        wallets
    }

    pub fn create_wallet(&mut self) -> String {
        let wallet = Wallet::new();
        let address = wallet.get_address();
        self.wallets.insert(address.clone(), wallet);
        self.save_to_file();
        address
    }

    pub fn get_addresses(&self) -> Vec<String> {
        let mut addresses = vec![];
        for (address, _) in &self.wallets {
            addresses.push(address.clone());
        }
        addresses
    }

    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        if let Some(wallet) = self.wallets.get(address) {
            return Some(wallet);
        }
        None
    }

    pub fn load_from_file(&mut self) {
        let path = current_dir().unwrap().join(WALLET_FILE);
        if !path.exists() {
            return;
        }

        let mut file = File::open(path).unwrap();
        let metadata = file.metadata().expect("Unable to read metadata");
        let mut buf = vec![0; metadata.len() as usize];
        let _ = file.read(&mut buf).expect("Unable to read file");
        let wallets = bincode::deserialize(&buf[..]).expect("Unable to deserialize wallets");
        self.wallets = wallets;
    }

    fn save_to_file(&self) {
        let path = current_dir().unwrap().join(WALLET_FILE);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)
            .expect("Unable to open wallets.dat");
        let mut writer = BufWriter::new(file);
        let wallets_bytes = bincode::serialize(&self.wallets).expect("Unable to serialize wallets");
        writer.write(wallets_bytes.as_slice()).unwrap();
        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use crate::Wallets;

    #[test]
    fn test_new_wallets() {
        let mut wallets = Wallets::new();
        let address = wallets.create_wallet();
        println!("The new wallet address is {}", address);
    }

    #[test]
    fn test_get_addresses() {
        let addresses = Wallets::new().get_addresses();

        println!("{:?}", addresses);
    }
}
