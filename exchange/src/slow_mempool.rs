use alloy_consensus::{Transaction, TxEnvelope, TypedTransaction};
use alloy_eips::Decodable2718;
use revm::context::TxEnv;

pub async fn add_evm_txn_from_hex(data: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from(""))
}

pub fn parse_raw_transaction(raw_tx: &[u8]) -> Result<TxEnv, Box<dyn std::error::Error>> {
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
