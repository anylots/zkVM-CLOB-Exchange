use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tiny_keccak::{Hasher, Sha3};
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub user_id: String,
    pub balances: HashMap<String, u64>, // token_id -> balance
}

impl Account {
    pub fn get_balance(&self, token_id: &str) -> u64 {
        *self.balances.get(token_id).unwrap_or(&0)
    }

    pub fn set_balance(&mut self, token_id: String, balance: u64) {
        self.balances.insert(token_id, balance);
    }

    // Calculate SHA3 hash for this account
    fn calculate_hash(&self) -> [u8; 32] {
        let mut sha3 = Sha3::v256();
        let mut output = [0u8; 32];

        // Hash user_id
        sha3.update(self.user_id.as_bytes());

        // Hash balances in a deterministic order (sorted by token_id)
        let mut sorted_balances: Vec<_> = self.balances.iter().collect();
        sorted_balances.sort_by_key(|(token_id, _)| *token_id);

        for (token_id, balance) in sorted_balances {
            sha3.update(token_id.as_bytes());
            sha3.update(&balance.to_le_bytes());
        }

        sha3.finalize(&mut output);
        output
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

pub struct State {
    pub user_balances: HashMap<String, HashMap<String, u64>>,
    pub merkle_tree: Option<MerkleNode>,
    pub state_root: Option<[u8; 32]>,
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

impl State {
    pub fn new() -> Self {
        State {
            user_balances: HashMap::new(),
            merkle_tree: None,
            state_root: None,
        }
    }

    // Helper method to get a user's balance for a specific token
    pub fn get_user_balance(&self, user_id: &str, token_id: &str) -> u64 {
        self.user_balances
            .get(user_id)
            .and_then(|balances| balances.get(token_id))
            .copied()
            .unwrap_or(0)
    }

    // Helper method to set a user's balance for a specific token
    pub fn set_user_balance(&mut self, user_id: String, token_id: String, balance: u64) {
        self.user_balances
            .entry(user_id)
            .or_insert_with(HashMap::new)
            .insert(token_id, balance);
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
            .map(|(user_id, balances)| calculate_user_hash(user_id, balances))
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
            self.merkle_tree = Some(root);
            self.state_root = Some(state_root);
            Some(state_root)
        } else {
            None
        }
    }
}

// Global State instance
lazy_static::lazy_static! {
    pub static ref STATE: Arc<RwLock<State>> = Arc::new(RwLock::new(State::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_state_root() {
        let mut state = State::new();

        // Create test user balances
        let mut user1_balances = HashMap::new();
        user1_balances.insert("BTC".to_string(), 100);
        user1_balances.insert("ETH".to_string(), 200);

        let mut user2_balances = HashMap::new();
        user2_balances.insert("BTC".to_string(), 50);

        state
            .user_balances
            .insert("user1".to_string(), user1_balances);
        state
            .user_balances
            .insert("user2".to_string(), user2_balances);

        // Generate state root
        let state_root = state.gen_state_root();

        assert!(state_root.is_some());
        assert!(state.merkle_tree.is_some());
        assert!(state.state_root.is_some());

        // Verify the root hash matches what's stored in the state
        assert_eq!(state_root, state.state_root);

        println!("Root hash: {:?}", state_root.unwrap());
    }

    #[test]
    fn test_empty_user_balances() {
        let mut state = State::new();
        // user_balances is already empty by default

        let state_root = state.gen_state_root();
        assert!(state_root.is_none());
    }

    #[test]
    fn test_no_user_balances() {
        let mut state = State::new();

        let state_root = state.gen_state_root();
        assert!(state_root.is_none());
    }

    #[test]
    fn test_helper_methods() {
        let mut state = State::new();

        // Test setting and getting user balances
        state.set_user_balance("user1".to_string(), "BTC".to_string(), 100);
        state.set_user_balance("user1".to_string(), "ETH".to_string(), 200);
        state.set_user_balance("user2".to_string(), "BTC".to_string(), 50);

        // Test getting balances
        assert_eq!(state.get_user_balance("user1", "BTC"), 100);
        assert_eq!(state.get_user_balance("user1", "ETH"), 200);
        assert_eq!(state.get_user_balance("user2", "BTC"), 50);
        assert_eq!(state.get_user_balance("user2", "ETH"), 0); // Should return 0 for non-existent token
        assert_eq!(state.get_user_balance("user3", "BTC"), 0); // Should return 0 for non-existent user

        // Test that state root generation still works
        let state_root = state.gen_state_root();
        assert!(state_root.is_some());
    }

    #[test]
    fn test_add_user_balance() {
        let mut state = State::new();

        // Test adding to non-existent user/token (should start from 0)
        state.add_user_balance("user1".to_string(), "BTC".to_string(), 50);
        assert_eq!(state.get_user_balance("user1", "BTC"), 50);

        // Test adding to existing balance
        state.add_user_balance("user1".to_string(), "BTC".to_string(), 25);
        assert_eq!(state.get_user_balance("user1", "BTC"), 75);

        // Test adding to different token for same user
        state.add_user_balance("user1".to_string(), "ETH".to_string(), 100);
        assert_eq!(state.get_user_balance("user1", "ETH"), 100);
        assert_eq!(state.get_user_balance("user1", "BTC"), 75); // Should not affect BTC balance

        // Test overflow protection (saturating_add)
        state.set_user_balance("user2".to_string(), "BTC".to_string(), u64::MAX - 10);
        state.add_user_balance("user2".to_string(), "BTC".to_string(), 20);
        assert_eq!(state.get_user_balance("user2", "BTC"), u64::MAX);
    }

    #[test]
    fn test_sub_user_balance() {
        let mut state = State::new();

        // Test subtracting from non-existent user/token (should fail)
        let result = state.sub_user_balance("user1".to_string(), "BTC".to_string(), 50);
        assert!(!result);
        assert_eq!(state.get_user_balance("user1", "BTC"), 0);

        // Set up initial balance
        state.set_user_balance("user1".to_string(), "BTC".to_string(), 100);

        // Test successful subtraction
        let result = state.sub_user_balance("user1".to_string(), "BTC".to_string(), 30);
        assert!(result);
        assert_eq!(state.get_user_balance("user1", "BTC"), 70);

        // Test subtraction of exact balance
        let result = state.sub_user_balance("user1".to_string(), "BTC".to_string(), 70);
        assert!(result);
        assert_eq!(state.get_user_balance("user1", "BTC"), 0);

        // Test insufficient balance
        let result = state.sub_user_balance("user1".to_string(), "BTC".to_string(), 10);
        assert!(!result);
        assert_eq!(state.get_user_balance("user1", "BTC"), 0); // Balance should remain unchanged

        // Test subtracting from different token
        state.set_user_balance("user1".to_string(), "ETH".to_string(), 200);
        let result = state.sub_user_balance("user1".to_string(), "ETH".to_string(), 50);
        assert!(result);
        assert_eq!(state.get_user_balance("user1", "ETH"), 150);
        assert_eq!(state.get_user_balance("user1", "BTC"), 0); // Should not affect BTC balance
    }
}
