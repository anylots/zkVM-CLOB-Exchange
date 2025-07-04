use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tiny_keccak::{Hasher, Sha3};

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
    pub account_list: Option<Vec<Account>>,
    pub merkle_tree: Option<MerkleNode>,
    pub state_root: Option<[u8; 32]>,
}

impl State {
    pub fn new() -> Self {
        State {
            account_list: None,
            merkle_tree: None,
            state_root: None,
        }
    }

    pub fn gen_state_root(&mut self) -> Option<[u8; 32]> {
        let accounts = self.account_list.as_ref()?;

        if accounts.is_empty() {
            return None;
        }

        // Calculate hash for each account
        let mut leaf_hashes: Vec<[u8; 32]> = accounts
            .iter()
            .map(|account| account.calculate_hash())
            .collect();

        // If odd number of accounts, duplicate the last hash to make it even
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_state_root() {
        let mut state = State::new();

        // Create test accounts
        let mut account1 = Account {
            user_id: "user1".to_string(),
            balances: HashMap::new(),
        };
        account1.set_balance("BTC".to_string(), 100);
        account1.set_balance("ETH".to_string(), 200);

        let mut account2 = Account {
            user_id: "user2".to_string(),
            balances: HashMap::new(),
        };
        account2.set_balance("BTC".to_string(), 50);

        state.account_list = Some(vec![account1, account2]);

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
    fn test_empty_account_list() {
        let mut state = State::new();
        state.account_list = Some(vec![]);

        let state_root = state.gen_state_root();
        assert!(state_root.is_none());
    }

    #[test]
    fn test_no_account_list() {
        let mut state = State::new();

        let state_root = state.gen_state_root();
        assert!(state_root.is_none());
    }
}
