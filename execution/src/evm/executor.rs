use crate::evm::storage::EvmDatabase;
use crate::evm::trie::calculate_trie_updates;
use alloy_primitives::map::foldhash::{HashMap, HashMapExt};
use alloy_primitives::{Address, B256, U256};
use alloy_rlp::{BufMut, Encodable};
use alloy_trie::Nibbles;
pub use alloy_trie::TrieAccount;
use alloy_trie::nodes::{LeafNode, TrieNode};
use revm::DatabaseCommit;
use revm::context::ContextTr;
use revm::database::{AccountState, CacheDB};
use revm::state::AccountInfo;
use revm::{Context, ExecuteEvm, MainBuilder, MainContext, context::TxEnv};
use tiny_keccak::{Hasher, Sha3};

pub const ACCOUNT_RLP_MAX_SIZE: usize = 110;

// AccountInfo and Storage changed after execute_block.
type PostState = (
    HashMap<B256, AccountInfo>,
    HashMap<B256, HashMap<B256, U256>>,
);

pub struct EvmExecutor<'a> {
    database: &'a mut CacheDB<EvmDatabase>,
    post_state: Option<PostState>,
}

impl<'a> EvmExecutor<'a> {
    pub fn new(database: &'a mut CacheDB<EvmDatabase>) -> Self {
        Self {
            database,
            post_state: None,
        }
    }

    pub fn execute_block(&mut self, block: Vec<TxEnv>) {
        for tx in block {
            let _ = self.execute_tx(tx);
        }
    }

    pub fn persistent(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let account_cache = &self.database.cache.accounts;
        let contract_cache = &self.database.cache.contracts;
        // Save account
        for (address, acc) in account_cache.iter() {
            if acc.account_state == AccountState::Touched {
                // No persistence required
                continue;
            }
            let acc_info = &acc.info;
            let acc_storage = &acc.storage;
            for (key, value) in acc_storage.iter() {
                self.database.db.save_storage(key, value);
            }
            self.database.db.save_account(address, acc_info);
        }
        // Save code
        for (address, code) in contract_cache.iter() {
            self.database
                .db
                .save_code(&Address::from_word(*address), code);
        }
        // Save hashed state and mpt trie.
        let mut hashed_accounts = HashMap::with_capacity(account_cache.len());
        let mut hashed_storage = HashMap::with_capacity(account_cache.len());

        let mut nodes_to_update: HashMap<Nibbles, TrieNode> = HashMap::new();

        for (address, acc) in account_cache.iter() {
            let hashed_address = keccak_address(address);
            let storage = acc
                .storage
                .iter()
                .map(|entry| (keccak_slot(entry.0), *entry.1))
                .collect::<HashMap<B256, U256>>();

            hashed_accounts.insert(hashed_address, acc.info.clone());
            hashed_storage.insert(hashed_address, storage);

            let mut account_rlp = Vec::with_capacity(ACCOUNT_RLP_MAX_SIZE);
            let trie_account = TrieAccount {
                nonce: acc.info.nonce,
                balance: acc.info.balance,
                storage_root: B256::default(),
                code_hash: acc.info.code_hash,
            };
            trie_account.encode(&mut account_rlp as &mut dyn BufMut);

            nodes_to_update.insert(
                Nibbles::unpack(hashed_address),
                TrieNode::Leaf(LeafNode::new(Nibbles::unpack(hashed_address), account_rlp)),
            );
        }
        self.post_state = Some((hashed_accounts, hashed_storage));

        let mut trie_updates: HashMap<Nibbles, TrieNode> = HashMap::new();

        for (address, _acc) in self.post_state.as_ref().unwrap().0.iter() {
            let target_path = Nibbles::unpack(address);
            let acc_trie_updates = self.fetch_trie_updates(target_path).unwrap_or_default();
            trie_updates.extend(acc_trie_updates);
        }

        calculate_trie_updates(nodes_to_update, &mut trie_updates);

        // Save Mpt node to DB.
        for trie_node in trie_updates.iter() {
            self.database
                .db
                .insert_account_trie_node(trie_node.0, trie_node.1);
        }

        Ok(())
    }

    fn fetch_trie_updates(&self, target_path: Nibbles) -> Option<HashMap<Nibbles, TrieNode>> {
        let mut current_path = Nibbles::default();
        let mut account_nodes = HashMap::new();
        loop {
            match self.database.db.get_account_trie_node(&current_path) {
                Some(TrieNode::Branch(node)) => {
                    account_nodes.insert(current_path.clone(), TrieNode::Branch(node.clone()));
                    let next_nibble = target_path.get(current_path.len()).unwrap();
                    if node.state_mask.is_bit_set(next_nibble) {
                        current_path.extend_from_slice(&vec![next_nibble]);
                    } else {
                        return None;
                    }
                }
                Some(TrieNode::EmptyRoot) => {
                    break;
                }
                Some(TrieNode::Extension(node)) => {
                    account_nodes.insert(current_path.clone(), TrieNode::Extension(node.clone()));
                    current_path.extend(&node.key);
                    if !target_path.to_vec().starts_with(&current_path.to_vec()) {
                        return None;
                    }
                }
                Some(TrieNode::Leaf(leaf_node)) => {
                    if target_path == current_path {
                        account_nodes
                            .insert(current_path.clone(), TrieNode::Leaf(leaf_node.clone()));
                        break;
                    }
                }
                None => {
                    break;
                }
            }
        }
        Some(account_nodes)
    }

    /// Execute a transaction using revm
    pub fn execute_tx(&mut self, tx: TxEnv) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Handler::run(&mut self, evm);
        // Validate
        if tx.gas_limit == 0 {
            return Err("Invalid gas limit: cannot be zero".into());
        }

        if tx.gas_price == 0u128 {
            return Err("Invalid gas price: cannot be zero".into());
        }

        let mut evm = Context::mainnet()
            .with_db(&mut self.database)
            .build_mainnet();
        let out = evm.transact(tx)?;

        let output_bytes = match out.result.output() {
            Some(output) => output.to_vec(),
            None => Vec::new(),
        };

        // Handle state finalization and commit properly
        let state = evm.ctx.journal_mut().finalize();
        evm.ctx.db_mut().commit(state);

        Ok(output_bytes)
    }
}

fn keccak_address(addr: &Address) -> B256 {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    sha3.update(&addr.to_vec());
    sha3.finalize(&mut output);
    B256::from(output)
}

fn keccak_slot(slot: &U256) -> B256 {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    sha3.update(&slot.to_be_bytes_vec());
    sha3.finalize(&mut output);
    B256::from(output)
}



#[cfg(test)]
mod test {
    use super::*;
    use alloy_primitives::B256;
    use alloy_primitives::{Address, Bytes, U256};
    use revm::primitives::TxKind;
    use revm::state::{AccountInfo, Bytecode};

    #[test]
    fn test_execute_tx() {
        // Create a legacy transaction (type 0) with minimal gas requirements
        let tx = TxEnv {
            tx_type: 0, // Legacy transaction type
            caller: Address::from([0x1; 20]),
            gas_limit: 21000,
            gas_price: 1u128,
            kind: TxKind::Call(Address::from([0x2; 20])),
            value: U256::ZERO,
            data: Bytes::new(),
            nonce: 0,
            chain_id: Some(1),
            access_list: vec![].into(),
            gas_priority_fee: None,
            blob_hashes: vec![],
            max_fee_per_blob_gas: 0,
            authorization_list: vec![],
        };

        let mut database = EvmDatabase::new();
        let account = AccountInfo::new(U256::from(100000), 0, B256::default(), Bytecode::default());
        database.persistent_db.set_account(
            &Address::from([0x1; 20]).to_vec(),
            serde_json::to_vec(&account).unwrap(),
        );
        let mut cache_db = CacheDB::<EvmDatabase>::new(database);

        let mut executor = EvmExecutor::new(&mut cache_db);
        let result = executor.execute_tx(tx);
        println!("result {:?}", result);

        let acc = executor
            .database
            .load_account(Address::from([0x2; 20]))
            .unwrap();
        println!("acc {:?}", acc);
    }
}
