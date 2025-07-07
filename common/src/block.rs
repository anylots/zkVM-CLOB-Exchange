use crate::traces::MatchedTrace;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub block_num: u128,
    pub txns: Vec<MatchedTrace>,
    pub txns_root: Option<[u8; 32]>,
    pub state_root: Option<[u8; 32]>,
}
