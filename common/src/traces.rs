use serde::{Deserialize, Serialize};

use crate::order::Order;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchedTrace {
    pub buy_order: Order,
    pub sell_order: Order,
    pub matched_amount: u64,
}
