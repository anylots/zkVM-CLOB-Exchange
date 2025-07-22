use anyhow::Result;
use revm::context::TxEnv;
use revm::database::CacheDB;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::evm::executor::EvmExecutor;
use crate::evm::mempool::EVM_MEMPOOL;
use crate::evm::storage::EvmDatabase;

static MAX_TXN_SIZE: u64 = 100;
static BLOCK_TIME_INTERVAL: Duration = Duration::from_millis(200);

pub struct BlockBuilder {
    pub block_db: sled::Db,
    pub state_db: CacheDB<EvmDatabase>,
    pub current_block_num: Arc<RwLock<u128>>,
    pub last_block_time: Arc<RwLock<Instant>>,
}

impl BlockBuilder {
    pub fn new(db_path: &str) -> Result<Self> {
        let block_db = sled::open(db_path)?;
        let database = EvmDatabase::new();
        let state_db = CacheDB::<EvmDatabase>::new(database);

        // Initialize block number from database or start from 0
        let current_block_num = match block_db.get("evm_latest_block_num")? {
            Some(bytes) => {
                let num_bytes: [u8; 16] = bytes
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid block number format"))?;
                u128::from_be_bytes(num_bytes)
            }
            None => 0,
        };

        Ok(BlockBuilder {
            block_db,
            state_db,
            current_block_num: Arc::new(RwLock::new(current_block_num)),
            last_block_time: Arc::new(RwLock::new(Instant::now())),
        })
    }

    pub async fn start_block_generation(&mut self) -> Result<()> {
        let mut pending_txns = Vec::new();

        loop {
            // Read new txns
            let poped_txns = {
                let mut mempool = EVM_MEMPOOL.write().await;
                mempool
                    .txns
                    .drain(1..MAX_TXN_SIZE as usize - pending_txns.len())
                    .collect::<Vec<_>>()
            };

            // Add new txns to pending
            pending_txns.extend(poped_txns);

            let should_generate_block = {
                let last_time = *self.last_block_time.read().await;
                let time_elapsed = last_time.elapsed() >= BLOCK_TIME_INTERVAL;
                let txn_count_reached = pending_txns.len() as u64 >= MAX_TXN_SIZE;

                (time_elapsed || txn_count_reached) && !pending_txns.is_empty()
            };

            if should_generate_block {
                // Generate and save block
                let block = self.create_block(pending_txns.clone()).await;
                self.save_block(&block).await;

                log::info!(
                    "Generated block #{} with {} transactions",
                    block.block_num,
                    block.txns.len()
                );

                // Clear pending traces and update last block time
                pending_txns.clear();
                *self.last_block_time.write().await = Instant::now();
            }

            // Sleep for a short interval before checking again
            sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn create_block(&mut self, txns: Vec<TxEnv>) -> Block {
        let mut executor = EvmExecutor::new(&mut self.state_db);
        executor.execute_block(txns);
        let block = Block {
            block_num: todo!(),
            txns,
            txns_root: todo!(),
            state_root: todo!(),
        };
        block
    }
    pub async fn save_block(&self, block: &Block) {}
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub block_num: u128,
    pub txns: Vec<TxEnv>,
    pub txns_root: Option<[u8; 32]>,
    pub state_root: Option<[u8; 32]>,
}
