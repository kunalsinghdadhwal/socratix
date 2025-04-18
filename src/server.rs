use crate::{
    Block, BlockInTransit, Blockchain, GLOBAL_CONFIG, MemoryPool, Nodes, Transaction, UTXOSet,
    block,
};

use std::error::Error;
use std::io::{BufReader, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use bincode::Result;
use clap::error;
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

fn send_tx(addr: &str, tx: &Transaction) {
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

fn serve(blockchain: Blockchain, stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let peer_addr = stream.peer_addr()?;
    let reader = BufReader::new(&stream);
    let pkg_reader = Deserializer::from_reader(reader).into_iter::<Package>();
    for pkg in pkg_reader {
        let pkg = pkg?;
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
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}
