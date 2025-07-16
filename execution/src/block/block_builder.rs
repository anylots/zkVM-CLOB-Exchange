use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tiny_keccak::{Hasher, Sha3};
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::exchange::MATCHED_TRACES;
use crate::exchange::STATE;
use common::block::Block;
use common::traces::MatchedTrace;

static MAX_TXN_SIZE: u64 = 100;
static BLOCK_TIME_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Clone, Debug)]
pub struct BlockBuilder {
    pub db: sled::Db,
    pub current_block_num: Arc<RwLock<u128>>,
    pub last_block_time: Arc<RwLock<Instant>>,
}

impl BlockBuilder {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = sled::open(db_path)?;

        // Initialize block number from database or start from 0
        let current_block_num = match db.get("latest_block_num")? {
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
            db,
            current_block_num: Arc::new(RwLock::new(current_block_num)),
            last_block_time: Arc::new(RwLock::new(Instant::now())),
        })
    }

    /// Async method to continuously monitor MATCHED_TRACES and generate blocks
    pub async fn start_block_generation(&self) -> Result<()> {
        let mut pending_traces = Vec::new();

        loop {
            // Read current matched traces
            let traces = {
                let mut traces_lock = MATCHED_TRACES.write().await;
                let current_traces = traces_lock.clone();
                traces_lock.clear(); // Clear processed traces
                current_traces
            };

            // Add new traces to pending
            pending_traces.extend(traces);

            let should_generate_block = {
                let last_time = *self.last_block_time.read().await;
                let time_elapsed = last_time.elapsed() >= BLOCK_TIME_INTERVAL;
                let txn_count_reached = pending_traces.len() as u64 >= MAX_TXN_SIZE;

                (time_elapsed || txn_count_reached) && !pending_traces.is_empty()
            };

            if should_generate_block {
                // Generate and save block
                // NOTE: Delayed block creation(async), using memory pool consensus?
                let block = self.create_block(pending_traces.clone()).await?;
                self.save_block(&block).await?;

                log::info!(
                    "Generated block #{} with {} transactions",
                    block.block_num,
                    block.txns.len()
                );

                // Clear pending traces and update last block time
                pending_traces.clear();
                *self.last_block_time.write().await = Instant::now();
            }

            // Sleep for a short interval before checking again
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Create a new block with the given transactions
    async fn create_block(&self, txns: Vec<MatchedTrace>) -> Result<Block> {
        let mut block_num_lock = self.current_block_num.write().await;
        *block_num_lock += 1;
        let block_num = *block_num_lock;
        drop(block_num_lock);

        {
            let mut state_db = STATE.write().await;
            for trace in &txns {
                let tokens: Vec<&str> = trace.buy_order.pair_id.split('_').collect();
                let base_token = tokens[0];
                let quote_token = tokens[1];

                state_db.state.add_user_balance(
                    trace.buy_order.user_id.clone(),
                    base_token.to_owned(),
                    trace.matched_amount,
                );
                state_db.state.sub_user_balance(
                    trace.sell_order.user_id.clone(),
                    base_token.to_owned(),
                    trace.matched_amount,
                );

                state_db.state.sub_user_balance(
                    trace.buy_order.user_id.clone(),
                    quote_token.to_owned(),
                    trace.matched_amount,
                );
                state_db.state.add_user_balance(
                    trace.sell_order.user_id.clone(),
                    quote_token.to_owned(),
                    trace.matched_amount,
                );

                // unfreeze
                state_db.state.unfreeze(
                    trace.buy_order.user_id.clone(),
                    quote_token.to_owned(),
                    trace.matched_amount,
                );
                state_db.state.unfreeze(
                    trace.sell_order.user_id.clone(),
                    base_token.to_owned(),
                    trace.matched_amount,
                );
            }
        }

        // Calc state root using read lock.
        let state_root = {
            let state_db = STATE.read().await;
            state_db.state.calculate_state_root()
        };

        // Calc txns root
        // NOTE: Refer to SUI or ETH/EIP-7862 to implement delayed state root calculation
        let txns_root = self.calculate_txns_root(&txns);

        Ok(Block {
            block_num,
            txns,
            txns_root: Some(txns_root),
            state_root: state_root,
        })
    }

    /// Save block to local storage using sled
    async fn save_block(&self, block: &Block) -> Result<()> {
        // Serialize block
        let block_data = serde_json::to_vec(block)
            .map_err(|e| anyhow::anyhow!("Failed to serialize block: {}", e))?;

        // Save block with key "block_{block_num}"
        let block_key = format!("block_{}", block.block_num);
        self.db.insert(block_key.as_bytes(), block_data)?;

        // Update latest block number
        let block_num_bytes = block.block_num.to_be_bytes();
        self.db.insert("latest_block_num", &block_num_bytes[..])?;

        // Flush to ensure data is persisted
        self.db.flush()?;

        Ok(())
    }

    /// Calculate txns root for the block (simplified implementation)
    fn calculate_txns_root(&self, txns: &[MatchedTrace]) -> [u8; 32] {
        let mut sha3 = Sha3::v256();
        let mut output = [0u8; 32];

        // Hash all transactions in the block
        for txn in txns {
            if let Ok(txn_data) = serde_json::to_vec(txn) {
                sha3.update(&txn_data);
            }
        }

        sha3.finalize(&mut output);
        output
    }

    /// Get a block by block number
    pub async fn get_block(&self, block_num: u128) -> Result<Option<Block>> {
        let block_key = format!("block_{}", block_num);

        match self.db.get(block_key.as_bytes())? {
            Some(block_data) => {
                let block: Block = serde_json::from_slice(&block_data)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize block: {}", e))?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    /// Get the latest block number
    pub async fn get_latest_block_num(&self) -> u128 {
        *self.current_block_num.read().await
    }

    /// Get all blocks in a range
    pub async fn get_blocks_range(&self, start: u128, end: u128) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();

        for block_num in start..=end {
            if let Some(block) = self.get_block(block_num).await? {
                blocks.push(block);
            }
        }

        Ok(blocks)
    }
}
