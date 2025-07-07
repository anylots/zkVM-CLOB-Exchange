use tokio::sync::RwLock;

use crate::STATE;
use crate::matching::{OrderBook, Trade};
use common::order::Order;
use std::collections::HashMap;
use std::sync::Arc;

// Global mempool state
pub struct Mempool {
    pub order_books: HashMap<String, OrderBook>, // pair_id -> OrderBook
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            order_books: HashMap::new(),
        }
    }

    pub async fn place_order(&mut self, order: Order) -> Result<String, String> {
        log::info!(
            "Processing order in mempool: id={}, user_id={}, pair_id={}, amount={}, price={}, side={}",
            order.id,
            order.user_id,
            order.pair_id,
            order.amount,
            order.price,
            if order.side { "buy" } else { "sell" }
        );

        // Parse pair_id to get base and quote tokens (e.g., "ETH_USDT")
        let tokens: Vec<&str> = order.pair_id.split('_').collect();
        if tokens.len() != 2 {
            log::error!(
                "Invalid pair format for order {}: {}",
                order.id,
                order.pair_id
            );
            return Err("Invalid pair format. Use BASE/QUOTE format".to_string());
        }

        let base_token = tokens[0];
        let quote_token = tokens[1];
        let state_db = STATE.read().await;

        // Check if user has sufficient balance
        if order.side {
            // Buy order: need quote token balance
            let user_balance = state_db.state.get_user_balance(&order.user_id, &quote_token);
            let required_balance = order.amount * order.price;
            if user_balance < required_balance {
                log::warn!(
                    "Insufficient quote token balance for order {}: required={}, available={}",
                    order.id,
                    required_balance,
                    user_balance
                );
                return Err("Insufficient quote token balance".to_string());
            }
        } else {
            // Sell order: need base token balance
            let user_balance = state_db.state.get_user_balance(&order.user_id, &base_token);
            if user_balance < order.amount {
                log::warn!(
                    "Insufficient base token balance for order {}: required={}, available={}",
                    order.id,
                    order.amount,
                    user_balance
                );
                return Err("Insufficient base token balance".to_string());
            }
        }

        // Get or create order book for this pair
        let order_book = self
            .order_books
            .entry(order.pair_id.clone())
            .or_insert_with(OrderBook::new);

        log::info!(
            "Adding order {} to order book for pair {}",
            order.id,
            order.pair_id
        );

        // Place order
        order_book.add_order(order.clone()).await;
        log::info!("Order {} processing completed successfully", order.id);

        Ok(order.id)
    }

    pub fn cancel_order(&mut self, pair_id: &str, order_id: &str) -> Result<Order, String> {
        if let Some(order_book) = self.order_books.get_mut(pair_id) {
            if let Some(cancelled_order) = order_book.cancel_order(order_id) {
                Ok(cancelled_order)
            } else {
                Err("Order not found".to_string())
            }
        } else {
            Err("Trading pair not found".to_string())
        }
    }

    pub fn get_order(&self, pair_id: &str, order_id: &str) -> Option<&Order> {
        self.order_books
            .get(pair_id)
            .and_then(|book| book.get_order(order_id))
    }

    pub fn get_order_book(&self, pair_id: &str) -> Option<&OrderBook> {
        self.order_books.get(pair_id)
    }

    pub fn get_trades(&self) -> Vec<Trade> {
        vec![]
    }
}

// Global mempool instance
lazy_static::lazy_static! {
    pub static ref MEMPOOL: Arc<RwLock<Mempool>> = Arc::new(RwLock::new(Mempool::new()));
}
