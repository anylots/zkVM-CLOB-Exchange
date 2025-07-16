use std::sync::Arc;

use alloy_consensus::{Transaction, TxEnvelope, TypedTransaction};
use alloy_eips::Decodable2718;
use revm::DatabaseCommit;
use revm::context::ContextTr;
use revm::database::{CacheDB, EmptyDB};
use revm::{Context, ExecuteEvm, MainBuilder, MainContext, context::TxEnv};
use tokio::sync::RwLock;

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
            gas_limit: tx.gas_limit,
            gas_price: tx.gas_price,
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

/// Execute a transaction using revm
pub fn execute_tx(tx: TxEnv) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Handler::run(&mut self, evm);
    // Validate
    if tx.gas_limit == 0 {
        return Err("Invalid gas limit: cannot be zero".into());
    }

    if tx.gas_price == 0u128 {
        return Err("Invalid gas price: cannot be zero".into());
    }

    let cache_db = CacheDB::<EmptyDB>::default();
    let mut evm = Context::mainnet().with_db(cache_db).build_mainnet();
    let out = evm.transact(tx)?;

    let output_bytes = match out.result.output() {
        Some(output) => output.to_vec(),
        None => Vec::new(),
    };

    // Handle state finalization and commit properly
    let state = evm.ctx.journal_mut().finalize();
    evm.ctx.db_mut().commit(state);

    Ok(output_bytes)
}

// Global evm mempool instance
lazy_static::lazy_static! {
    pub static ref EVM_MEMPOOL: Arc<RwLock<Mempool>> = Arc::new(RwLock::new(Mempool::new()));
}
