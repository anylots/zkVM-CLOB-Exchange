use crate::order::Order;
use crate::matching::{OrderBook, Trade};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Global exchange state
pub struct Exchange {
    pub order_books: HashMap<String, OrderBook>, // pair_id -> OrderBook
    pub user_balances: HashMap<String, HashMap<String, u64>>, // user_id -> token -> balance
    pub trades: Vec<Trade>,
}

impl Exchange {
    pub fn new() -> Self {
        Self {
            order_books: HashMap::new(),
            user_balances: HashMap::new(),
            trades: Vec::new(),
        }
    }

    pub fn deposit(&mut self, user_id: String, token: String, amount: u64) -> Result<(), String> {
        let user_balances = self.user_balances.entry(user_id).or_insert_with(HashMap::new);
        let balance = user_balances.entry(token).or_insert(0);
        *balance += amount;
        Ok(())
    }

    pub fn withdraw(&mut self, user_id: String, token: String, amount: u64) -> Result<(), String> {
        let user_balances = self.user_balances.entry(user_id).or_insert_with(HashMap::new);
        let balance = user_balances.entry(token).or_insert(0);
        
        if *balance < amount {
            return Err("Insufficient balance".to_string());
        }
        
        *balance -= amount;
        Ok(())
    }

    pub fn get_balance(&self, user_id: &str, token: &str) -> u64 {
        self.user_balances
            .get(user_id)
            .and_then(|balances| balances.get(token))
            .copied()
            .unwrap_or(0)
    }

    pub fn place_order(&mut self, order: Order) -> Result<Vec<Trade>, String> {
        log::info!("Processing order in mempool: id={}, user_id={}, pair_id={}, amount={}, price={}, side={}", 
                   order.id, order.user_id, order.pair_id, order.amount, order.price, 
                   if order.side { "buy" } else { "sell" });
        
        // Parse pair_id to get base and quote tokens (e.g., "ETH/USDT")
        let tokens: Vec<&str> = order.pair_id.split('/').collect();
        if tokens.len() != 2 {
            log::error!("Invalid pair format for order {}: {}", order.id, order.pair_id);
            return Err("Invalid pair format. Use BASE/QUOTE format".to_string());
        }
        
        let base_token = tokens[0];
        let quote_token = tokens[1];

        // Check if user has sufficient balance
        if order.side {
            // Buy order: need quote token balance
            let required_balance = order.amount * order.price / 1_000_000; // Assuming price is in micro units
            let user_balance = self.get_balance(&order.user_id, quote_token);
            log::debug!("Buy order balance check: user_id={}, required={}, available={}", 
                       order.user_id, required_balance, user_balance);
            if user_balance < required_balance {
                log::warn!("Insufficient quote token balance for order {}: required={}, available={}", 
                          order.id, required_balance, user_balance);
                return Err("Insufficient quote token balance".to_string());
            }
        } else {
            // Sell order: need base token balance
            let user_balance = self.get_balance(&order.user_id, base_token);
            log::debug!("Sell order balance check: user_id={}, required={}, available={}", 
                       order.user_id, order.amount, user_balance);
            if user_balance < order.amount {
                log::warn!("Insufficient base token balance for order {}: required={}, available={}", 
                          order.id, order.amount, user_balance);
                return Err("Insufficient base token balance".to_string());
            }
        }

        // Get or create order book for this pair
        let order_book = self.order_books.entry(order.pair_id.clone()).or_insert_with(OrderBook::new);
        
        log::info!("Adding order {} to order book for pair {}", order.id, order.pair_id);
        
        // Place order and get trades
        let trades = order_book.add_order(order.clone());
        
        log::info!("Order {} matching completed: {} trades generated", order.id, trades.len());
        
        // Log each trade
        for trade in &trades {
            log::info!("Trade executed: buy_order={}, sell_order={}, price={}, quantity={}", 
                      trade.buy_order_id, trade.sell_order_id, trade.price, trade.quantity);
        }
        
        // Store trades
        self.trades.extend(trades.clone());
        
        log::info!("Order {} processing completed successfully", order.id);
        
        Ok(trades)
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

    pub fn get_trades(&self) -> &Vec<Trade> {
        &self.trades
    }
}

// Global exchange instance
lazy_static::lazy_static! {
    pub static ref EXCHANGE: Arc<Mutex<Exchange>> = Arc::new(Mutex::new(Exchange::new()));
}
