use tokio::sync::RwLock;

use crate::exchange::STATE;
use crate::exchange::matching::{OrderBook, Trade};
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

        let mut state_db = STATE.write().await;

        let user_id = order.user_id.clone();
        let base_token = order.token_a.clone();
        let quote_token = &order.token_b.clone();

        // Check if user has sufficient balance
        if order.side {
            let user_balance = state_db.state.get_user_balance(&user_id, quote_token);
            let frozen_balance = state_db.state.get_frozen(user_id.clone(), quote_token);
            // Overflow checking
            let order_cost = order
                .amount
                .checked_mul(order.price)
                .ok_or("Arithmetic overflow: order amount * price too large")?;
            let required_balance = order_cost
                .checked_add(frozen_balance)
                .ok_or("Arithmetic overflow: total required balance too large")?;

            if user_balance < required_balance {
                log::warn!(
                    "Insufficient quote token balance for order {}: required={}, available={}",
                    order.id,
                    required_balance,
                    user_balance
                );
                return Err("Insufficient quote token balance".to_string());
            }
            state_db
                .state
                .freeze(user_id, quote_token.to_owned(), required_balance);
        } else {
            // Sell order: need base token balance
            let user_balance = state_db.state.get_user_balance(&user_id, &base_token);
            if user_balance < order.amount {
                log::warn!(
                    "Insufficient base token balance for order {}: required={}, available={}",
                    order.id,
                    order.amount,
                    user_balance
                );
                return Err("Insufficient base token balance".to_string());
            }
            state_db
                .state
                .freeze(user_id, base_token.to_owned(), order.amount);
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

    pub async fn cancel_order(&mut self, pair_id: &str, order_id: &str) -> Result<Order, String> {
        if let Some(order_book) = self.order_books.get_mut(pair_id) {
            if let Some(cancelled_order) = order_book.cancel_order(order_id) {
                let user_id = cancelled_order.user_id.clone();
                let base_token = cancelled_order.token_a.clone();
                let quote_token = cancelled_order.token_b.clone();

                let mut state_db = STATE.write().await;
                if cancelled_order.side {
                    state_db.state.unfreeze(
                        user_id,
                        quote_token,
                        cancelled_order.remaining_amount(),
                    );
                } else {
                    state_db.state.unfreeze(
                        user_id,
                        base_token,
                        cancelled_order.remaining_amount(),
                    );
                }
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