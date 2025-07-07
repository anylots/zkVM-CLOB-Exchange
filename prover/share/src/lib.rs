use std::collections::HashMap;

use common::{
    block::Block,
    state::{Account, State},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkVMInput {
    pub blocks: Vec<Block>,
    pub state: State,
}

pub fn load() -> State {
    let db = sled::open("state_db").unwrap();
    if let Ok(Some(data)) = db.get("prev_state") {
        if let Ok(user_balances) = serde_json::from_slice::<HashMap<String, Account>>(&data) {
            return State {
                user_balances,
                state_root: None,
            };
        }
    }
    State {
        user_balances: HashMap::new(),
        state_root: None,
    }
}

pub fn load_blocks(start: u64, length: u64) -> Option<Vec<Block>> {
    let db = sled::open("block_db").ok()?;
    let mut blocks = vec![];
    for i in start..start + length {
        if let Ok(Some(data)) = db.get(format!("user_balances_{}", i)) {
            if let Ok(block) = serde_json::from_slice::<Block>(&data) {
                blocks.push(block);
            }
        } else {
            return None;
        }
    }
    Some(blocks)
}
