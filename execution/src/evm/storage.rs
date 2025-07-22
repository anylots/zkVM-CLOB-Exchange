use alloy_primitives::Address;
use alloy_primitives::B256;
use alloy_primitives::U256;
use alloy_rlp::Decodable;
use alloy_trie::Nibbles;
use alloy_trie::nodes::TrieNode;
use revm::database::DBErrorMarker;
use revm::database::{Database, DatabaseRef};
use revm::primitives::StorageKey;
use revm::primitives::StorageValue;
use revm::state::AccountInfo;
use revm::state::Bytecode;
use sled::Tree;

pub struct PersistentDb {
    pub(crate) account_table: Tree,
    pub(crate) code_table: Tree,
    pub(crate) storage_table: Tree,
    pub(crate) account_trie: Tree,
    pub(crate) storage_trie: Tree,
}

impl PersistentDb {
    pub fn new(
        account_table: Tree,
        code_table: Tree,
        storage_table: Tree,
        account_trie: Tree,
        storage_trie: Tree,
    ) -> Self {
        Self {
            account_table,
            code_table,
            storage_table,
            account_trie,
            storage_trie,
        }
    }

    pub fn get_account(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.account_table
            .get(key)
            .ok()
            .flatten()
            .map(|v| v.to_vec())
    }

    pub fn set_account(&mut self, key: &[u8], value: Vec<u8>) {
        self.account_table.insert(key, value).unwrap();
    }

    pub fn get_code(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.code_table.get(key).ok().flatten().map(|v| v.to_vec())
    }

    pub fn set_code(&mut self, key: &[u8], value: Vec<u8>) {
        self.code_table.insert(key, value).unwrap();
    }

    pub fn get_storage(&self, key: &[u8]) -> Option<Vec<u8>> {
        Some(self.storage_table.get(key).unwrap()?.to_vec())
    }

    pub fn set_storage(&mut self, key: &[u8], value: Vec<u8>) {
        self.storage_table.insert(key, value).unwrap();
    }

    pub fn set_account_trie_node(&mut self, key: &[u8], value: Vec<u8>) {
        self.account_trie.insert(key, value).unwrap();
    }

    pub fn get_account_trie_node(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.account_trie
            .get(key)
            .ok()
            .flatten()
            .map(|v| v.to_vec())
    }
}

pub struct EvmDatabase {
    pub account_infos: Vec<AccountInfo>,
    pub persistent_db: PersistentDb,
}

impl EvmDatabase {
    pub fn new() -> Self {
        let db = sled::open("evm_db").unwrap();

        let persistent_db = PersistentDb::new(
            db.open_tree("account_table").unwrap(),
            db.open_tree("code_table").unwrap(),
            db.open_tree("storage_table").unwrap(),
            db.open_tree("account_tree").unwrap(),
            db.open_tree("storage_tree").unwrap(),
        );

        Self {
            account_infos: vec![],
            persistent_db: persistent_db,
        }
    }

    pub fn save_account(&mut self, address: &Address, account: &AccountInfo) {
        self.persistent_db
            .set_account(&address.to_vec(), serde_json::to_vec(account).unwrap());
    }

    pub fn save_code(&mut self, address: &Address, code: &Bytecode) {
        self.persistent_db
            .set_code(&address.to_vec(), code.bytes_slice().to_vec());
    }

    pub fn save_storage(&mut self, key: &StorageKey, value: &StorageValue) {
        self.persistent_db
            .set_storage(&key.to_be_bytes_vec(), value.to_be_bytes_vec());
    }

    pub fn insert_account_trie_node(&mut self, key: &Nibbles, node: &TrieNode) {
        let mut rlp_data = Vec::new();
        let _rlp_node = node.rlp(&mut rlp_data);
        self.persistent_db
            .set_account_trie_node(&key.to_vec(), rlp_data);
    }

    pub fn get_account_trie_node(&self, key: &Nibbles) -> Option<TrieNode> {
        let rlp_value = self
            .persistent_db
            .get_account_trie_node(&key.to_vec())
            .unwrap();
        Some(TrieNode::decode(&mut rlp_value.as_slice()).ok()?)
    }
}

impl Database for EvmDatabase {
    type Error = DatabaseError;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.basic_ref(address)
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.code_by_hash_ref(code_hash)
    }

    fn storage(
        &mut self,
        address: Address,
        index: StorageKey,
    ) -> Result<StorageValue, Self::Error> {
        self.storage_ref(address, index)
    }

    fn block_hash(&mut self, _number: u64) -> Result<B256, Self::Error> {
        self.block_hash_ref(_number)
    }
}

impl DatabaseRef for EvmDatabase {
    type Error = DatabaseError;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        match self.persistent_db.get_account(address.as_slice()) {
            Some(data) => {
                let account =
                    serde_json::from_slice(&data).map_err(|e| DatabaseError::DbError(e.into()))?;
                Ok(Some(account))
            }
            None => Ok(None), // Account doesn't exist
        }
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        match self.persistent_db.get_code(code_hash.as_slice()) {
            Some(data) => {
                let code =
                    serde_json::from_slice(&data).map_err(|e| DatabaseError::DbError(e.into()))?;
                Ok(code)
            }
            None => Ok(Bytecode::default()), // Code doesn't exist
        }
    }

    fn storage_ref(
        &self,
        address: Address,
        index: StorageKey,
    ) -> Result<StorageValue, Self::Error> {
        let slot = storage_key(&address, &index);

        match self.persistent_db.get_storage(&slot) {
            Some(value) => Ok(U256::from_be_slice(&value)),
            None => Ok(U256::default()),
        }
    }

    fn block_hash_ref(&self, _number: u64) -> Result<B256, Self::Error> {
        Ok(B256::default())
    }
}

fn storage_key(contract_address: &Address, slot: &StorageKey) -> Vec<u8> {
    let mut key = Vec::with_capacity(64); // 32 + 32 字节
    key.extend_from_slice(contract_address.as_slice());
    key.extend_from_slice(&slot.to_be_bytes_vec());
    key
}

/// Bundled errors variants thrown by various db.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("{0}")]
    DbError(eyre::Error),
}

impl DBErrorMarker for DatabaseError {}
