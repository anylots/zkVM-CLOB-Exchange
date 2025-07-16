use crate::evm::handle_evm_request;
use crate::exchange::STATE;
use crate::exchange::matching::Trade;
use crate::exchange::mempool::MEMPOOL;
use axum::{
    Router, extract::Json, http::StatusCode, response::Json as ResponseJson, routing::post,
};
use common::order::Order;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[derive(Deserialize)]
pub struct DepositRequest {
    pub user_id: String,
    pub token: String,
    pub amount: u64,
}

#[derive(Deserialize)]
pub struct WithdrawRequest {
    pub user_id: String,
    pub token: String,
    pub amount: u64,
}

#[derive(Deserialize)]
pub struct PlaceOrderRequest {
    pub user_id: String,
    pub pair_id: String,
    pub amount: u64,
    pub price: u64,
    pub side: bool, // true for buy, false for sell
}

#[derive(Deserialize)]
pub struct CancelOrderRequest {
    pub pair_id: String,
    pub order_id: String,
}

#[derive(Deserialize)]
pub struct GetBalanceRequest {
    pub user_id: String,
    pub token: String,
}

#[derive(Deserialize)]
pub struct GetOrderRequest {
    pub pair_id: String,
    pub order_id: String,
}

#[derive(Deserialize)]
pub struct GetOrderBookRequest {
    pub pair_id: String,
}

#[derive(Deserialize)]
pub struct SubmitEvmTxnRequest {
    pub rlp_data: String, // Hex-encoded RLP transaction data
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct PlaceOrderResponse {
    pub order_id: String,
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: u64,
}

#[derive(Serialize)]
pub struct OrderBookResponse {
    pub best_bid: Option<u64>,
    pub best_ask: Option<u64>,
}

#[derive(Serialize)]
pub struct SubmitEvmTxnResponse {
    pub tx_hash: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

pub async fn start() {
    // Create exchange API router
    let exchange_app = create_exchange_router();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let exchange_app = exchange_app.layer(cors);

    // Create EVM API router
    let evm_app = create_evm_router();
    let evm_cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let evm_app = evm_app.layer(evm_cors);

    // Start exchange server on port 3030
    let exchange_addr: SocketAddr = "[::1]:3030".parse().unwrap();
    log::info!("Exchange server running on http://{}", exchange_addr);
    let exchange_listener = tokio::net::TcpListener::bind(exchange_addr).await.unwrap();

    // Start EVM server on port 8545
    let evm_addr: SocketAddr = "[::1]:8545".parse().unwrap();
    log::info!("EVM server running on http://{}", evm_addr);
    let evm_listener = tokio::net::TcpListener::bind(evm_addr).await.unwrap();

    // Run both servers concurrently
    tokio::select! {
        _ = axum::serve(exchange_listener, exchange_app) => {},
        _ = axum::serve(evm_listener, evm_app) => {},
    }
}

fn create_exchange_router() -> Router {
    Router::new()
        .route("/deposit", post(handle_deposit))
        .route("/withdraw", post(handle_withdraw))
        .route("/order/place", post(handle_place_order))
        .route("/order/cancel", post(handle_cancel_order))
        .route("/balance", post(handle_get_balance))
        .route("/order/get", post(handle_get_order))
        .route("/orderbook", post(handle_get_orderbook))
        .route("/trades", post(handle_get_trades))
}

fn create_evm_router() -> Router {
    Router::new().route("/", post(handle_evm_request))
}

async fn handle_deposit(
    Json(request): Json<DepositRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, StatusCode> {
    log::info!(
        "Deposit request: user_id={}, token={}, amount={}",
        request.user_id,
        request.token,
        request.amount
    );

    let mut state_db = STATE.write().await;

    state_db.state.add_user_balance(
        request.user_id.clone(),
        request.token.clone(),
        request.amount,
    );
    Ok(ResponseJson(ApiResponse::success(())))
}

async fn handle_withdraw(
    Json(request): Json<WithdrawRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, StatusCode> {
    log::info!(
        "Withdraw request: user_id={}, token={}, amount={}",
        request.user_id,
        request.token,
        request.amount
    );

    let mut state_db = STATE.write().await;

    state_db.state.sub_user_balance(
        request.user_id.clone(),
        request.token.clone(),
        request.amount,
    );
    Ok(ResponseJson(ApiResponse::success(())))
}

async fn handle_place_order(
    Json(request): Json<PlaceOrderRequest>,
) -> Result<ResponseJson<ApiResponse<PlaceOrderResponse>>, StatusCode> {
    log::info!(
        "Received order request: user_id={}, pair_id={}, amount={}, price={}, side={}",
        request.user_id,
        request.pair_id,
        request.amount,
        request.price,
        if request.side { "buy" } else { "sell" }
    );

    let mut mempool = MEMPOOL.write().await;

    // Generate unique order ID
    let order_id = format!("order_{}", rand::random::<u64>());

    let order = Order::new(
        order_id.clone(),
        request.user_id,
        request.pair_id,
        request.amount,
        request.price,
        request.side,
    );

    log::info!(
        "Created order: id={}, user_id={}, pair_id={}",
        order_id,
        order.user_id,
        order.pair_id
    );

    match mempool.place_order(order.clone()).await {
        Ok(order_id) => {
            log::info!("Order processed successfully: order_id = {}", order_id,);
            let response = PlaceOrderResponse { order_id };
            Ok(ResponseJson(ApiResponse::success(response)))
        }
        Err(e) => {
            log::error!("Failed to process order: id={}, error={}", order_id, e);
            Ok(ResponseJson(ApiResponse::error(e)))
        }
    }
}

async fn handle_cancel_order(
    Json(request): Json<CancelOrderRequest>,
) -> Result<ResponseJson<ApiResponse<Order>>, StatusCode> {
    log::info!(
        "Cancel order request: pair_id={}, order_id={}",
        request.pair_id,
        request.order_id
    );

    let mut mempool = MEMPOOL.write().await;
    match mempool
        .cancel_order(&request.pair_id, &request.order_id)
        .await
    {
        Ok(cancelled_order) => {
            log::info!(
                "Order cancelled successfully: pair_id={}, order_id={}",
                request.pair_id,
                request.order_id
            );
            Ok(ResponseJson(ApiResponse::success(cancelled_order)))
        }
        Err(e) => {
            log::error!(
                "Failed to cancel order: pair_id={}, order_id={}, error={}",
                request.pair_id,
                request.order_id,
                e
            );
            Ok(ResponseJson(ApiResponse::error(e)))
        }
    }
}

async fn handle_get_balance(
    Json(request): Json<GetBalanceRequest>,
) -> Result<ResponseJson<ApiResponse<BalanceResponse>>, StatusCode> {
    let state_db = STATE.read().await;
    let balance = state_db
        .state
        .get_user_balance(&request.user_id, &request.token);
    let response = BalanceResponse { balance };

    Ok(ResponseJson(ApiResponse::success(response)))
}

async fn handle_get_order(
    Json(request): Json<GetOrderRequest>,
) -> Result<ResponseJson<ApiResponse<Order>>, StatusCode> {
    let mempool = MEMPOOL.read().await;

    match mempool.get_order(&request.pair_id, &request.order_id) {
        Some(order) => Ok(ResponseJson(ApiResponse::success(order.clone()))),
        None => Ok(ResponseJson(ApiResponse::error(
            "Order not found".to_string(),
        ))),
    }
}

async fn handle_get_orderbook(
    Json(request): Json<GetOrderBookRequest>,
) -> Result<ResponseJson<ApiResponse<OrderBookResponse>>, StatusCode> {
    let mempool = MEMPOOL.read().await;

    match mempool.get_order_book(&request.pair_id) {
        Some(order_book) => {
            let response = OrderBookResponse {
                best_bid: order_book.get_best_bid(),
                best_ask: order_book.get_best_ask(),
            };
            Ok(ResponseJson(ApiResponse::success(response)))
        }
        None => Ok(ResponseJson(ApiResponse::error(
            "Trading pair not found".to_string(),
        ))),
    }
}

async fn handle_get_trades() -> Result<ResponseJson<ApiResponse<Vec<Trade>>>, StatusCode> {
    let mempool = MEMPOOL.read().await;
    let trades = mempool.get_trades().clone();
    Ok(ResponseJson(ApiResponse::success(trades)))
}
