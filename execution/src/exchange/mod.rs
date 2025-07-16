pub mod matching;
pub mod mempool;

use std::sync::Arc;

use common::{state::StateDB, traces::MatchedTrace};
use tokio::sync::RwLock;

// Global traces instance
lazy_static::lazy_static! {
    pub static ref MATCHED_TRACES: Arc<RwLock<Vec<MatchedTrace>>> = Arc::new(RwLock::new(vec![]));
}

// Global State instance
lazy_static::lazy_static! {
    pub static ref STATE: Arc<RwLock<StateDB>> = Arc::new(RwLock::new(StateDB::new("state_db")));
}
