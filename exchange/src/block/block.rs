use crate::matched_traces::MatchedTrace;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub block_num: u128,
    pub txns: Vec<MatchedTrace>,
    pub state_root: Option<[u8; 32]>,
}
