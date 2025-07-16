pub mod mempool;

use axum::{extract::Json, http::StatusCode, response::Json as ResponseJson};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::evm::mempool::EVM_MEMPOOL;

#[derive(Deserialize)]
pub struct EvmRequest {
    pub method: String,
    pub params: Vec<Value>,
    pub id: Option<Value>,
}

#[derive(Serialize)]
pub struct EvmResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

impl EvmResponse {
    pub fn success(result: Value, id: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(error: Value, id: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

pub async fn handle_evm_request(
    Json(request): Json<EvmRequest>,
) -> Result<ResponseJson<EvmResponse>, StatusCode> {
    log::info!(
        "EVM request: method={}, params={}",
        request.method,
        json!(request.params)
    );

    let id = request.id.clone();

    match request.method.as_str() {
        "eth_getTransactionCount" => {
            // response: return transaction count as 0x1
            let result = json!("0x1");
            Ok(ResponseJson(EvmResponse::success(result, id)))
        }
        "eth_gasPrice" => {
            // response: return gas price as 0x3b9aca00 (1 Gwei)
            let result = json!("0x3b9aca00");
            Ok(ResponseJson(EvmResponse::success(result, id)))
        }
        "eth_getBalance" => {
            // response: return balance as 0x3635c9adc5dea00000 (1000 ETH in wei)
            let result = json!("0x3635c9adc5dea00000");
            Ok(ResponseJson(EvmResponse::success(result, id)))
        }
        "eth_chainId" => {
            // response: return chain ID as 0x539 (1337, common for local dev)
            let result = json!("0x539");
            Ok(ResponseJson(EvmResponse::success(result, id)))
        }
        "eth_blockNumber" => {
            // response: return blockNumber as 0x1a (26)
            let result = json!("0x10d4f");
            Ok(ResponseJson(EvmResponse::success(result, id)))
        }
        "eth_sendRawTransaction" => {
            let mut mempool = EVM_MEMPOOL.write().await;
            match mempool
                .add_evm_txn(&request.params.first().unwrap().as_str().unwrap_or_default())
                .await
            {
                Ok(tx_hash) => {
                    let tx_hash_hex = format!("0x{}", hex::encode(tx_hash));
                    log::info!(
                        "Add evm transaction successfully: tx_hash = {}",
                        tx_hash_hex
                    );
                    return Ok(ResponseJson(EvmResponse::success(json!(tx_hash_hex), id)));
                }
                Err(e) => {
                    log::error!("Failed to process EVM transaction: error={}", e);
                    return Ok(ResponseJson(EvmResponse::error(json!(e.to_string()), id)));
                }
            }
        }
        "eth_getTransactionReceipt" => {
            // response: return null (transaction not found)
            let result = json!(null);
            Ok(ResponseJson(EvmResponse::success(result, id)))
        }
        "eth_getBlockByNumber" => {
            let block_number = request.params.get(0).cloned().unwrap_or(json!("latest"));
            let full_transactions = request
                .params
                .get(1)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let block = json!({
                "number": block_number,
                "hash": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef",
                "parentHash": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdee",
                "nonce": "0x0000000000000000",
                "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
                "logsBloom": "0x0",
                "transactionsRoot": "0x0",
                "stateRoot": "0x0",
                "receiptsRoot": "0x0",
                "miner": "0x0000000000000000000000000000000000000000",
                "difficulty": "0x1",
                "totalDifficulty": "0x1",
                "extraData": "0x",
                "size": "0x1",
                "gasLimit": "0x1",
                "gasUsed": "0x0",
                "timestamp": "0x5bad55",
                "transactions": if full_transactions {
                    json!([{
                        "hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                        "nonce": "0x0",
                        "blockHash": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef",
                        "blockNumber": block_number,
                        "transactionIndex": "0x0",
                        "from": "0x0000000000000000000000000000000000000000",
                        "to": "0x0000000000000000000000000000000000000000",
                        "value": "0x0",
                        "gas": "0x5208",
                        "gasPrice": "0x3b9aca00",
                        "input": "0x"
                    }])
                } else {
                    json!(["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"])
                },
                "uncles": json!([])
            });

            Ok(ResponseJson(EvmResponse::success(block, id)))
        }
        _ => {
            let error = json!({
                "code": -32601,
                "message": "Method not found"
            });
            Ok(ResponseJson(EvmResponse::error(error, id)))
        }
    }
}
