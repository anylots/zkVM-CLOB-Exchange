use serde::{Deserialize, Serialize};
use std::{collections::HashMap};
use tiny_keccak::{Hasher, Sha3};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub balances: HashMap<String, u64>, // token_id -> balance
}

impl Account {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
        }
    }
    pub fn get_balance(&self, token_id: &str) -> u64 {
        *self.balances.get(token_id).unwrap_or(&0)
    }

    pub fn set_balance(&mut self, token_id: String, balance: u64) {
        self.balances.insert(token_id, balance);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub user_balances: HashMap<String, Account>,
    pub state_root: Option<[u8; 32]>,
}

pub struct StateDB {
    pub db: sled::Db,
    pub state: State,
}

impl StateDB {
    pub fn new(db_path: &str) -> Self {
        let db = sled::open(db_path).unwrap();
        StateDB {
            db,
            state: State::new(),
        }
    }

    pub fn save(&self) {
        let serialized = serde_json::to_vec(&self.state.user_balances).unwrap();
        self.db.insert("user_balances", serialized).unwrap();
    }

    pub fn load(&mut self) {
        if let Ok(Some(data)) = self.db.get("user_balances") {
            if let Ok(user_balances) = serde_json::from_slice::<HashMap<String, Account>>(&data) {
                self.state.user_balances = user_balances;
            }
        }
    }
}
impl State {
    pub fn new() -> Self {
        State {
            user_balances: HashMap::new(),
            state_root: None,
        }
    }

    // Helper method to get a user's balance for a specific token
    pub fn get_user_balance(&self, user_id: &str, token_id: &str) -> u64 {
        self.user_balances
            .get(user_id)
            .and_then(|balances| Some(balances.get_balance(token_id)))
            .unwrap_or(0)
    }

    // Helper method to set a user's balance for a specific token
    pub fn set_user_balance(&mut self, user_id: String, token_id: String, balance: u64) {
        self.user_balances
            .entry(user_id)
            .or_insert_with(|| Account::new())
            .set_balance(token_id, balance);
    }

    // Helper method to add to a user's balance for a specific token
    pub fn add_user_balance(&mut self, user_id: String, token_id: String, amount: u64) {
        let current_balance = self.get_user_balance(&user_id, &token_id);
        let new_balance = current_balance.saturating_add(amount);
        self.set_user_balance(user_id, token_id, new_balance);
    }

    // Helper method to subtract from a user's balance for a specific token
    // Returns true if successful, false if insufficient balance
    pub fn sub_user_balance(&mut self, user_id: String, token_id: String, amount: u64) -> bool {
        let current_balance = self.get_user_balance(&user_id, &token_id);
        if current_balance >= amount {
            let new_balance = current_balance - amount;
            self.set_user_balance(user_id, token_id, new_balance);
            true
        } else {
            false
        }
    }

    pub fn gen_state_root(&mut self) -> Option<[u8; 32]> {
        if self.user_balances.is_empty() {
            return None;
        }

        // Calculate hash for each user's balances
        let mut leaf_hashes: Vec<[u8; 32]> = self
            .user_balances
            .iter()
            .map(|(user_id, account)| calculate_user_hash(user_id, &account.balances))
            .collect();

        // If odd number of users, duplicate the last hash to make it even
        if leaf_hashes.len() % 2 == 1 {
            leaf_hashes.push(leaf_hashes[leaf_hashes.len() - 1]);
        }

        // Build binary tree bottom-up
        let mut nodes: Vec<MerkleNode> =
            leaf_hashes.into_iter().map(MerkleNode::new_leaf).collect();

        // Build tree level by level until we have one root node
        while nodes.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..nodes.len()).step_by(2) {
                let left = nodes[i].clone();
                let right = if i + 1 < nodes.len() {
                    nodes[i + 1].clone()
                } else {
                    // If odd number of nodes, duplicate the last one
                    left.clone()
                };

                next_level.push(MerkleNode::new_internal(left, right));
            }

            nodes = next_level;
        }

        // Store the tree and root hash
        if let Some(root) = nodes.into_iter().next() {
            let state_root = root.hash;
            Some(state_root)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

impl MerkleNode {
    fn new_leaf(hash: [u8; 32]) -> Self {
        MerkleNode {
            hash,
            left: None,
            right: None,
        }
    }

    fn new_internal(left: MerkleNode, right: MerkleNode) -> Self {
        let mut sha3 = Sha3::v256();
        let mut output = [0u8; 32];

        sha3.update(&left.hash);
        sha3.update(&right.hash);
        sha3.finalize(&mut output);

        MerkleNode {
            hash: output,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

// Helper function to calculate hash for a user's balances
fn calculate_user_hash(user_id: &str, balances: &HashMap<String, u64>) -> [u8; 32] {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    // Hash user_id
    sha3.update(user_id.as_bytes());

    // Hash balances in a deterministic order (sorted by token_id)
    let mut sorted_balances: Vec<_> = balances.iter().collect();
    sorted_balances.sort_by_key(|(token_id, _)| *token_id);

    for (token_id, balance) in sorted_balances {
        sha3.update(token_id.as_bytes());
        sha3.update(&balance.to_le_bytes());
    }

    sha3.finalize(&mut output);
    output
}
