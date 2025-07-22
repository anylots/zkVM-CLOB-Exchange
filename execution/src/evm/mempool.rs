use std::sync::Arc;

use alloy_consensus::{Transaction, TxEnvelope, TypedTransaction};
use alloy_eips::Decodable2718;
use alloy_primitives::{Address, Bytes, U256};
use revm::DatabaseCommit;
use revm::context::ContextTr;
use revm::context::TxEnv;
use tokio::sync::RwLock;

use crate::evm::storage::EvmDatabase;

pub struct Mempool {
    pub txns: Vec<TxEnv>,
}

impl Mempool {
    pub fn new() -> Self {
        Self { txns: Vec::new() }
    }

    pub async fn add_evm_txn(&mut self, param: &str) -> Result<String, Box<dyn std::error::Error>> {
        let txn = parse_raw_transaction(param.as_bytes())?;
        self.txns.push(txn);
        Ok(String::from(""))
    }
}

fn parse_raw_transaction(raw_tx: &[u8]) -> Result<TxEnv, Box<dyn std::error::Error>> {
    let mut data = raw_tx;
    let transaction = TxEnvelope::decode_2718(&mut &mut data)?.into_typed_transaction();

    let tx = match transaction {
        TypedTransaction::Legacy(tx) => Some(TxEnv {
            tx_type: 0,
            caller: tx.to.into_to().unwrap_or_default(),
            gas_limit: tx.gas_limit.try_into().unwrap_or(21000u64),
            gas_price: tx.gas_price.try_into().unwrap_or(0u128),
            kind: tx.kind(),
            value: tx.value,
            data: tx.input,
            nonce: tx.nonce,
            chain_id: tx.chain_id,
            access_list: vec![].into(),
            gas_priority_fee: None,
            blob_hashes: vec![],
            max_fee_per_blob_gas: 0,
            authorization_list: vec![],
        }),
        _ => None,
    };

    Ok(tx.unwrap())
}

// Global evm mempool instance
lazy_static::lazy_static! {
    pub static ref EVM_MEMPOOL: Arc<RwLock<Mempool>> = Arc::new(RwLock::new(Mempool::new()));
}
