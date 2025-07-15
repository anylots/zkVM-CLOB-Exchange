#![no_main]
sp1_zkvm::entrypoint!(main);
use std::vec;

use common::traces::MatchedTrace;
use share::ZkVMInput;
use tiny_keccak::{Hasher, Sha3};

pub fn main() {
    // Read the input.
    let x = sp1_zkvm::io::read::<ZkVMInput>();

    let blocks = x.blocks;
    let mut state = x.state;
    let prev_state_root = blocks.first().unwrap().state_root.unwrap_or_default();
    let post_state_root = blocks.last().unwrap().state_root.unwrap_or_default();

    // state
    let lastest_state_root = state.calculate_state_root().unwrap_or_default();

    assert!(
        prev_state_root == lastest_state_root,
        "prev_state_root == lastest_state_root"
    );

    let mut txns_roots: Vec<[u8; 32]> = vec![];

    for block in blocks {
        for trace in block.txns.clone() {
            let tokens: Vec<&str> = trace.buy_order.pair_id.split('_').collect();
            let base_token = tokens[0];
            let quote_token = tokens[1];
            state.add_user_balance(
                trace.buy_order.user_id.clone(),
                base_token.to_owned(),
                trace.matched_amount,
            );
            state.sub_user_balance(
                trace.sell_order.user_id.clone(),
                base_token.to_owned(),
                trace.matched_amount,
            );

            state.sub_user_balance(
                trace.buy_order.user_id.clone(),
                quote_token.to_owned(),
                trace.matched_amount,
            );
            state.add_user_balance(
                trace.sell_order.user_id.clone(),
                quote_token.to_owned(),
                trace.matched_amount,
            );
        }
        // Calculate current block state root
        let block_post_state_root = state.calculate_state_root().unwrap_or_default();
        assert!(
            block_post_state_root == block.state_root.unwrap_or_default(),
            "block_post_state_root == block.state_root"
        );

        // Calculate txns root
        let txns_root = calculate_txns_root(&block.txns);
        assert!(
            txns_root == block.txns_root.unwrap_or_default(),
            "txns_root == block.txns_root"
        );
        txns_roots.push(txns_root);
    }

    let da_hash = calculate_da_hash(&txns_roots);

    // calculate pi hash
    let pi_hash = calculate_pi_hash(&prev_state_root, &post_state_root, &da_hash);

    // Commit to the public values of the program. The final proof will have a commitment to all the
    // bytes that were committed to.
    sp1_zkvm::io::commit(&pi_hash);
}

/// Calculate txns root for the block
fn calculate_txns_root(txns: &[MatchedTrace]) -> [u8; 32] {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    // Hash all transactions in the block
    for txn in txns {
        if let Ok(txn_data) = serde_json::to_vec(txn) {
            sha3.update(&txn_data);
        }
    }

    sha3.finalize(&mut output);
    output
}

// Helper function to calculate hash with all blocks' txns for DA.
fn calculate_da_hash(txns_roots: &Vec<[u8; 32]>) -> [u8; 32] {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    for txns_root in txns_roots {
        sha3.update(txns_root);
    }

    sha3.finalize(&mut output);
    output
}

// Helper function to calculate public input for zk proof.
fn calculate_pi_hash(
    prev_state_root: &[u8; 32],
    post_state_root: &[u8; 32],
    da_hash: &[u8; 32],
) -> [u8; 32] {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    sha3.update(prev_state_root);
    sha3.update(post_state_root);
    sha3.update(da_hash);

    sha3.finalize(&mut output);
    output
}
