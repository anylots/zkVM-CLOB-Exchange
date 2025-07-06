use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::order::Order;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchedTrace {
    pub buy_order: Order,
    pub sell_order: Order,
    pub matched_amount: u64,
}

// Global traces instance
lazy_static::lazy_static! {
    pub static ref MATCHED_TRACES: Arc<RwLock<Vec<MatchedTrace>>> = Arc::new(RwLock::new(vec![]));
}
