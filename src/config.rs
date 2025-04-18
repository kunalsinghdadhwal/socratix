use std::collections::HashMap;
use std::env;
use std::sync::RwLock;

use once_cell::sync::Lazy;

pub static GLOBAL_CONFIG: Lazy<Config> = Lazy::new(|| Config::new());

static DEFAULT_NODE_ADDR: &str = "127.0.0.1:42069";

const NODE_ADDRESS_KEY: &str = "NODE_ADDRESS";
const MINING_ADDRESS_KEY: &str = "MINING_ADDRESS";

pub struct Config {
    inner: RwLock<HashMap<String, String>>,
}

impl Config {
    pub fn new() -> Config {
        let mut node_adddr = String::from(DEFAULT_NODE_ADDR);

        if let Ok(addr) = env::var("NODE_ADDRESS") {
            node_adddr = addr;
        }

        let mut map = HashMap::new();
        map.insert(String::from(NODE_ADDRESS_KEY), node_adddr);

        Config {
            inner: RwLock::new(map),
        }
    }

    pub fn get_node_addr(&self) -> String {
        let inner = self.inner.read().unwrap();
        inner.get(NODE_ADDRESS_KEY).unwrap().clone()
    }

    pub fn set_mining_addr(&self, addr: String) {
        let mut inner = self.inner.write().unwrap();
        inner.insert(String::from(MINING_ADDRESS_KEY), addr);
    }

    pub fn get_mining_addr(&self) -> Option<String> {
        let inner = self.inner.read().unwrap();

        if let Some(addr) = inner.get(MINING_ADDRESS_KEY) {
            return Some(addr.clone());
        }
        None
    }

    pub fn is_miner(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.contains_key(MINING_ADDRESS_KEY)
    }
}

#[cfg(test)]
mod tests {
    use super::NODE_ADDRESS_KEY;
    use crate::Config;
    use std::env;

    #[test]
    fn new_config() {
        unsafe {
            env::set_var(NODE_ADDRESS_KEY, "127.0.0.1:2002");
        }

        let config = Config::new();
        let node_addr = config.get_node_addr();
        println!("{}", node_addr)
    }
}
