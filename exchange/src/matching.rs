use crate::MATCHED_TRACES;
use common::order::{Order, OrderStatus};
use common::traces::MatchedTrace;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Trade {
    pub buy_order_id: String,
    pub sell_order_id: String,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: u64,
}

// Wrapper for orders in buy heap (max heap by price, then FIFO by time)
#[derive(Clone, Debug)]
struct BuyOrder(Order);

impl PartialEq for BuyOrder {
    fn eq(&self, other: &Self) -> bool {
        self.0.price == other.0.price && self.0.created_at == other.0.created_at
    }
}

impl Eq for BuyOrder {}

impl PartialOrd for BuyOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BuyOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher price first, then earlier time (FIFO)
        match self.0.price.cmp(&other.0.price) {
            Ordering::Equal => other.0.created_at.cmp(&self.0.created_at), // Earlier time first
            other => other,                                                // Higher price first
        }
    }
}

// Wrapper for orders in sell heap (min heap by price, then FIFO by time)
#[derive(Clone, Debug)]
struct SellOrder(Order);

impl PartialEq for SellOrder {
    fn eq(&self, other: &Self) -> bool {
        self.0.price == other.0.price && self.0.created_at == other.0.created_at
    }
}

impl Eq for SellOrder {}

impl PartialOrd for SellOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SellOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower price first, then earlier time (FIFO)
        match other.0.price.cmp(&self.0.price) {
            Ordering::Equal => other.0.created_at.cmp(&self.0.created_at), // Earlier time first
            other => other,                                                // Lower price first
        }
    }
}

pub struct OrderBook {
    buy_orders: BinaryHeap<BuyOrder>,
    sell_orders: BinaryHeap<SellOrder>,
    pub order_map: HashMap<String, Order>, // order_id -> Order for quick lookup
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            buy_orders: BinaryHeap::new(),
            sell_orders: BinaryHeap::new(),
            order_map: HashMap::new(),
        }
    }

    fn is_order_cancelled(&self, order_id: &str) -> bool {
        self.order_map
            .get(order_id)
            .map(|order| matches!(order.status, OrderStatus::Cancelled))
            .unwrap_or(true)
    }

    pub async fn add_order(&mut self, mut order: Order) {
        let order_id = order.id.clone();
        let order_side = order.side;
        let order_amount = order.amount;
        let order_price = order.price;

        log::info!(
            "Adding order to order book: id={}, side={}, amount={}, price={}",
            order_id,
            if order_side { "buy" } else { "sell" },
            order_amount,
            order_price
        );

        if order.side {
            // Buy order - match against sell orders
            log::debug!("Matching buy order {} against sell orders", order_id);
            self.match_buy_order(&mut order).await;
            let remaining = order.remaining_amount();
            if remaining > 0 {
                self.order_map.insert(order.id.clone(), order.clone());
                self.buy_orders.push(BuyOrder(order));
            } else {
                log::info!("Buy order {} fully filled", order_id);
            }
        } else {
            // Sell order - match against buy orders
            log::debug!("Matching sell order {} against buy orders", order_id);
            self.match_sell_order(&mut order).await;
            let remaining = order.remaining_amount();
            if remaining > 0 {
                self.order_map.insert(order.id.clone(), order.clone());
                self.sell_orders.push(SellOrder(order));
            } else {
                log::info!("Sell order {} fully filled", order_id);
            }
        }
    }

    async fn match_buy_order(&mut self, buy_order: &mut Order) {
        let mut updated_sells = Vec::new();

        let mut traces = MATCHED_TRACES.write().await;

        while let Some(SellOrder(mut sell_order)) = self.sell_orders.pop() {
            // Skip cancelled orders
            if self.is_order_cancelled(&sell_order.id) {
                continue;
            }

            if sell_order.price > buy_order.price {
                // No match possible, put back and break
                self.sell_orders.push(SellOrder(sell_order));
                break;
            }

            let trade_quantity =
                std::cmp::min(buy_order.remaining_amount(), sell_order.remaining_amount());
            let _trade_price = sell_order.price; // Price-time priority: use maker's price

            // MatchedTrace
            traces.push(MatchedTrace {
                buy_order: buy_order.clone(),
                sell_order: sell_order.clone(),
                matched_amount: trade_quantity,
            });

            // Update orders
            buy_order.fill(trade_quantity);
            sell_order.fill(trade_quantity);

            // Update order in map
            self.order_map
                .insert(sell_order.id.clone(), sell_order.clone());

            if sell_order.remaining_amount() > 0 {
                updated_sells.push(SellOrder(sell_order));
            }

            if buy_order.remaining_amount() == 0 {
                break;
            }
        }

        // Put back unmatched sell orders
        for sell in updated_sells {
            self.sell_orders.push(sell);
        }
    }

    async fn match_sell_order(&mut self, sell_order: &mut Order) {
        let mut updated_buys = Vec::new();

        let mut traces = MATCHED_TRACES.write().await;

        while let Some(BuyOrder(mut buy_order)) = self.buy_orders.pop() {
            // Skip and drop cancelled orders
            if self.is_order_cancelled(&buy_order.id) {
                continue;
            }

            if buy_order.price < sell_order.price {
                // No match possible, put back and break
                self.buy_orders.push(BuyOrder(buy_order));
                break;
            }

            let trade_quantity =
                std::cmp::min(sell_order.remaining_amount(), buy_order.remaining_amount());

            // MatchedTrace
            traces.push(MatchedTrace {
                buy_order: buy_order.clone(),
                sell_order: sell_order.clone(),
                matched_amount: trade_quantity,
            });

            // Update orders
            sell_order.fill(trade_quantity);
            buy_order.fill(trade_quantity);

            // Update order in map
            self.order_map
                .insert(buy_order.id.clone(), buy_order.clone());

            if buy_order.remaining_amount() > 0 {
                updated_buys.push(BuyOrder(buy_order));
            }

            if sell_order.remaining_amount() == 0 {
                break;
            }
        }

        // Put back unmatched buy orders
        for buy in updated_buys {
            self.buy_orders.push(buy);
        }
    }

    pub fn cancel_order(&mut self, order_id: &str) -> Option<Order> {
        log::info!("Attempting to cancel order: {}", order_id);

        // Check if order exists and get its side
        let (order_side, already_cancelled) = if let Some(order) = self.order_map.get(order_id) {
            (order.side, matches!(order.status, OrderStatus::Cancelled))
        } else {
            log::warn!("Order {} not found for cancellation", order_id);
            return None;
        };

        // Check if already cancelled
        if already_cancelled {
            log::warn!("Order {} is already cancelled", order_id);
            return self.order_map.get(order_id).cloned();
        }

        if let Some(order) = self.order_map.get_mut(order_id) {
            log::info!(
                "Order {} found, cancelling. Side: {}, remaining: {}",
                order_id,
                if order_side { "buy" } else { "sell" },
                order.remaining_amount()
            );

            // This order will be skipped (pop) when matching (lazy removal).
            order.set_status(OrderStatus::Cancelled);
            let cancelled_order = order.clone();

            log::info!("Order {} successfully cancelled", order_id);
            Some(cancelled_order)
        } else {
            log::warn!("Order {} disappeared during cancellation", order_id);
            None
        }
    }

    pub fn get_order(&self, order_id: &str) -> Option<&Order> {
        self.order_map.get(order_id)
    }

    pub fn get_best_bid(&self) -> Option<u64> {
        for buy_order in &self.buy_orders {
            if !self.is_order_cancelled(&buy_order.0.id) {
                return Some(buy_order.0.price);
            }
        }
        None
    }

    pub fn get_best_ask(&self) -> Option<u64> {
        for sell_order in &self.sell_orders {
            if !self.is_order_cancelled(&sell_order.0.id) {
                return Some(sell_order.0.price);
            }
        }
        None
    }
}
