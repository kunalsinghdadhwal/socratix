use crate::{
    Block, BlockInTransit, Blockchain, GLOBAL_CONFIG, MemoryPool, Nodes, Transaction, UTXOSet,
};

use std::error::Error;
use std::io::{BufReader, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use data_encoding::HEXLOWER;
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

const NODE_VERSION: usize = 1;
pub const CENTRAL_NODE: &str = "127.0.0.1:42069";

pub const TRANSACTION_THRESHOLD: usize = 2;

static GLOBAL_NODES: Lazy<Nodes> = Lazy::new(|| {
    let nodes = Nodes::new();
    nodes.add_node(CENTRAL_NODE.to_string());
    nodes
});

static GLOBAL_MEMORY_POOL: Lazy<MemoryPool> = Lazy::new(|| MemoryPool::new());

static GLOBAL_BLOCKS_IN_TRANSIT: Lazy<BlockInTransit> = Lazy::new(|| BlockInTransit::new());

const TCP_WRITE_TIMEOUT: u64 = 1000;

pub struct Server {
    blockchain: Blockchain,
}

impl Server {
    pub fn new(blockchain: Blockchain) -> Self {
        Server { blockchain }
    }

    pub fn run(&self, addr: &str) {
        let listener = TcpListener::bind(addr).unwrap();

        if addr.eq(CENTRAL_NODE) == false {
            let best_height = self.blockchain.get_best_height();
            info!("Send version best height: {}", best_height);
            send_version(CENTRAL_NODE, best_height);
        }
        info!("Listening on {}", addr);
        for stream in listener.incoming() {
            let blockchain = self.blockchain.clone();
            thread::spawn(|| match stream {
                Ok(stream) => {
                    if let Err(e) = serve(blockchain, stream) {
                        error!("Error on serving client: {}", e);
                    }
                }
                Err(e) => {
                    error!("Connection failed: {}", e);
                }
            });
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OpType {
    Tx,
    Block,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Package {
    Block {
        addr_from: String,
        block: Vec<u8>,
    },
    GetBlocks {
        addr_from: String,
    },
    GetData {
        addr_from: String,
        op_type: OpType,
        id: Vec<u8>,
    },
    Inv {
        addr_from: String,
        op_type: OpType,
        items: Vec<Vec<u8>>,
    },
    Tx {
        addr_from: String,
        transaction: Vec<u8>,
    },
    Version {
        addr_from: String,
        version: usize,
        best_height: usize,
    },
}

fn send_get_data(addr: &str, op_type: OpType, id: &[u8]) {
    let socket_addr = addr.parse().unwrap();
    let node_addr = GLOBAL_CONFIG.get_node_addr().parse().unwrap();
    send_data(
        socket_addr,
        Package::GetData {
            addr_from: node_addr,
            op_type,
            id: id.to_vec(),
        },
    );
}

fn send_inv(addr: &str, op_type: OpType, blocks: &[Vec<u8>]) {
    let socket_addr = addr.parse().unwrap();
    let node_addr = GLOBAL_CONFIG.get_node_addr().parse().unwrap();
    send_data(
        socket_addr,
        Package::Inv {
            addr_from: node_addr,
            op_type,
            items: blocks.to_vec(),
        },
    );
}

fn send_block(addr: &str, block: &Block) {
    let socket_addr = addr.parse().unwrap();
    let node_addr = GLOBAL_CONFIG.get_node_addr().parse().unwrap();
    send_data(
        socket_addr,
        Package::Block {
            addr_from: node_addr,
            block: block.serialize(),
        },
    );
}

pub fn send_tx(addr: &str, tx: &Transaction) {
    let socket_addr = addr.parse().unwrap();
    let node_addr = GLOBAL_CONFIG.get_node_addr().parse().unwrap();
    send_data(
        socket_addr,
        Package::Tx {
            addr_from: node_addr,
            transaction: tx.serialize(),
        },
    );
}

fn send_version(addr: &str, height: usize) {
    let socket_addr = addr.parse().unwrap();
    let node_addr = GLOBAL_CONFIG.get_node_addr().parse().unwrap();
    send_data(
        socket_addr,
        Package::Version {
            addr_from: node_addr,
            version: NODE_VERSION,
            best_height: height,
        },
    );
}

fn send_get_blocks(addr: &str) {
    let socket_addr = addr.parse().unwrap();
    let node_addr = GLOBAL_CONFIG.get_node_addr().parse().unwrap();
    send_data(
        socket_addr,
        Package::GetBlocks {
            addr_from: node_addr,
        },
    );
}

fn serve(blockchain: Blockchain, stream: TcpStream) -> std::result::Result<(), Box<dyn Error>> {
    let peer_addr = stream.peer_addr()?;
    let reader = BufReader::new(&stream);
    let pkg_reader = Deserializer::from_reader(reader).into_iter::<Package>();

    for pkg in pkg_reader {
        let pkg = pkg.unwrap();
        info!("Receive requesr from {}: {:?}", peer_addr, pkg);

        match pkg {
            Package::GetBlocks { addr_from } => {
                let blocks = blockchain.get_block_hashes();
                send_inv(addr_from.as_str(), OpType::Block, &blocks);
            }
            Package::Version {
                addr_from,
                version,
                best_height,
            } => {
                info!("version = {}, best_height = {}", version, best_height);
                let local_best_height = blockchain.get_best_height();

                if local_best_height < best_height {
                    send_get_blocks(addr_from.as_str());
                }

                if local_best_height > best_height {
                    send_version(addr_from.as_str(), blockchain.get_best_height());
                }

                if GLOBAL_NODES.node_is_known(peer_addr.to_string().as_str()) == false {
                    GLOBAL_NODES.add_node(addr_from);
                }
            }
            Package::GetData {
                addr_from,
                op_type,
                id,
            } => match op_type {
                OpType::Block => {
                    if let Some(block) = blockchain.get_block(id.as_slice()) {
                        send_block(addr_from.as_str(), &block);
                    }
                }
                OpType::Tx => {
                    let txid_hex = HEXLOWER.encode(id.as_slice());

                    if let Some(tx) = GLOBAL_MEMORY_POOL.get(txid_hex.as_str()) {
                        send_tx(addr_from.as_str(), &tx);
                    }
                }
            },
            Package::Block { addr_from, block } => {
                let block = Block::deserialize(block.as_slice());
                blockchain.add_block(&block);

                info!("Added block: {}", block.get_hash());

                if GLOBAL_BLOCKS_IN_TRANSIT.len() > 0 {
                    let block_hash = GLOBAL_BLOCKS_IN_TRANSIT.first().unwrap();

                    send_get_data(addr_from.as_str(), OpType::Block, &block_hash);
                    GLOBAL_BLOCKS_IN_TRANSIT.remove(block_hash.as_slice());
                } else {
                    let utxo_set = UTXOSet::new(blockchain.clone());
                    utxo_set.reindex();
                }
            }
            Package::Inv {
                addr_from,
                op_type,
                items,
            } => match op_type {
                OpType::Block => {
                    GLOBAL_BLOCKS_IN_TRANSIT.add_blocks(items.as_slice());
                    let block_hash = items.get(0).unwrap();

                    send_get_data(addr_from.as_str(), OpType::Block, block_hash);
                    GLOBAL_BLOCKS_IN_TRANSIT.remove(block_hash);
                }
                OpType::Tx => {
                    let txid = items.get(0).unwrap();
                    let txid_hex = HEXLOWER.encode(txid);

                    if GLOBAL_MEMORY_POOL.containes(txid_hex.as_str()) == false {
                        send_get_data(addr_from.as_str(), OpType::Tx, txid);
                    }
                }
            },
            Package::Tx {
                addr_from,
                transaction,
            } => {
                let tx = Transaction::deserialize(transaction.as_slice());
                let txid = tx.get_id_bytes();

                GLOBAL_MEMORY_POOL.add(tx);

                let node_addr = GLOBAL_CONFIG.get_node_addr();

                if node_addr.eq(CENTRAL_NODE) {
                    let nodes = GLOBAL_NODES.get_nodes();

                    for node in &nodes {
                        if node_addr.eq(node.get_addr().as_str()) {
                            continue;
                        }

                        if addr_from.eq(node.get_addr().as_str()) {
                            continue;
                        }

                        send_inv(node.get_addr().as_str(), OpType::Tx, &vec![txid.clone()]);
                    }
                }

                if GLOBAL_MEMORY_POOL.len() >= TRANSACTION_THRESHOLD && GLOBAL_CONFIG.is_miner() {
                    let mining_address = GLOBAL_CONFIG.get_mining_addr().unwrap();
                    let coinbase_tx = Transaction::new_coinbase_tx(mining_address.as_str());
                    let mut txs = GLOBAL_MEMORY_POOL.get_all();

                    txs.push(coinbase_tx);

                    let new_block = blockchain.mine_block(&txs);
                    let utxo_set = UTXOSet::new(blockchain.clone());
                    utxo_set.reindex();

                    info!("New Block Mined: {}", new_block.get_hash());

                    for tx in &txs {
                        let txid_hex = HEXLOWER.encode(tx.get_id());
                        GLOBAL_MEMORY_POOL.remove(txid_hex.as_str());
                    }

                    let nodes = GLOBAL_NODES.get_nodes();

                    for node in &nodes {
                        if node_addr.eq(node.get_addr().as_str()) {
                            continue;
                        }

                        send_inv(
                            node.get_addr().as_str(),
                            OpType::Block,
                            &vec![new_block.get_hash_bytes()],
                        );
                    }
                }
            }
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

fn send_data(addr: SocketAddr, pkg: Package) {
    info!("Send Package: {:?}", &pkg);
    let stream = TcpStream::connect(addr);

    if stream.is_err() {
        error!("The {} is not valid", addr);

        GLOBAL_NODES.evict_node(addr.to_string().as_str());
        return;
    }

    let mut stream = stream.unwrap();
    let _ = stream.set_read_timeout(Option::from(Duration::from_millis(TCP_WRITE_TIMEOUT)));
    let _ = serde_json::to_writer(&stream, &pkg);
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use crate::server::{OpType, send_get_data};
    use data_encoding::HEXLOWER;

    #[test]
    fn test_send_get_block() {
        send_get_data(
            "127.0.0.1:2001",
            OpType::Block,
            "00f95a9ca28526e95e94f2eda7d3c6559f41a30b184991d5ccc036de7b134408".as_bytes(),
        );
    }

    #[test]
    fn test_send_get_transaction() {
        let txid = HEXLOWER
            .decode("164651291115cbf132f6c3e2a9729a84b0eb29da4481b7dfcd1e1b9e708cb6fa".as_bytes())
            .unwrap();
        send_get_data("127.0.0.1:2001", OpType::Tx, &txid);
    }

    #[test]
    fn test_vec() {
        let a = vec![6, 1, 2, 3, 5, 4];
        println!("{:?}", a);
    }
}
