use alloy_primitives::{
    B256,
    map::foldhash::{HashMap, HashMapExt},
};
use alloy_rlp::{BufMut, Encodable};
use alloy_trie::{
    Nibbles,
    nodes::{BranchNode, ExtensionNode, RlpNode, TrieNode},
};
use tiny_keccak::{Hasher, Sha3};

type StateCommitment = alloy_trie::proof::DecodedProofRetainer;

pub fn calculate_trie_updates(
    mut nodes_to_update: HashMap<Nibbles, TrieNode>,
    trie_updates: &mut HashMap<Nibbles, TrieNode>,
) {
    while !nodes_to_update.is_empty() {
        let mut next_to_update: HashMap<Nibbles, TrieNode> = HashMap::new();
        for (path, node) in nodes_to_update.iter() {
            if path.len() > 0 {
                let path_vec = path.to_vec();
                let parent_vec = path_vec[0..path_vec.len() - 1].to_vec();
                let parent_path = Nibbles::from_nibbles(parent_vec);
                let parent_node = trie_updates.get(&parent_path).unwrap();
                match parent_node {
                    TrieNode::Branch(branch_node) => {
                        let child_index = path_vec[path_vec.len()];

                        // Create child_rlp
                        let new_child_rlp = node_to_rlp(node);

                        // Create a new branch node with updated stack and state_mask
                        if let Some(child_rlp) = new_child_rlp {
                            let mut new_stack = branch_node.stack.clone();
                            let mut new_state_mask = branch_node.state_mask;

                            let mut stack_index = 0;
                            for i in 0..child_index {
                                if new_state_mask.is_bit_set(i) {
                                    stack_index += 1;
                                }
                            }
                            if new_state_mask.is_bit_set(child_index) {
                                new_stack[stack_index] = child_rlp;
                            } else {
                                new_stack.insert(stack_index, child_rlp);
                                new_state_mask.set_bit(child_index);
                            }

                            let new_branch = BranchNode::new(new_stack, new_state_mask);
                            let new_branch_node = TrieNode::Branch(new_branch);

                            next_to_update.insert(parent_path.clone(), new_branch_node.clone());
                            trie_updates.insert(parent_path, new_branch_node);
                        }
                    }
                    TrieNode::EmptyRoot => {}
                    TrieNode::Extension(extension_node) => {
                        let new_child_rlp = node_to_rlp(node);

                        // Create a new extension node
                        if let Some(child_rlp) = new_child_rlp {
                            let new_extension =
                                ExtensionNode::new(extension_node.key.clone(), child_rlp);
                            let new_extension_node = TrieNode::Extension(new_extension);

                            // Add to both next_to_update and trie_updates
                            next_to_update.insert(parent_path.clone(), new_extension_node.clone());
                            trie_updates.insert(parent_path, new_extension_node);
                        }
                    }
                    TrieNode::Leaf(_leaf_node) => {}
                }
            }
        }
        nodes_to_update = next_to_update;
    }
}

fn node_to_rlp(node: &TrieNode) -> Option<RlpNode> {
    let new_child_rlp = match node {
        TrieNode::Leaf(leaf_node) => {
            // Encode the leaf node to get its RLP representation
            let mut encoded = Vec::new();
            leaf_node.encode(&mut encoded as &mut dyn BufMut);
            RlpNode::from_raw(&encoded)
        }
        TrieNode::Branch(branch) => {
            let mut encoded = Vec::new();
            branch.encode(&mut encoded as &mut dyn BufMut);
            RlpNode::from_raw(&encoded)
        }
        TrieNode::Extension(ext) => {
            let mut encoded = Vec::new();
            ext.encode(&mut encoded as &mut dyn BufMut);
            RlpNode::from_raw(&encoded)
        }
        TrieNode::EmptyRoot => None,
    };
    new_child_rlp
}

fn keccak_node(encoded_node: &Vec<u8>) -> B256 {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    sha3.update(encoded_node);
    sha3.finalize(&mut output);
    B256::from(output)
}
