use super::transaction::{Transaction, TransactionError, TransactionId};
use crate::identity::NodeId;
use crate::protocol::Signature64;
use anonnet_common::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A block in the consensus chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,

    /// Transactions in this block
    pub transactions: Vec<Transaction>,

    /// Validator signatures (node_id -> signature)
    pub signatures: HashMap<NodeId, Signature64>,
}

impl Block {
    /// Create a new block
    pub fn new(
        height: u64,
        previous_hash: [u8; 32],
        proposer: NodeId,
        transactions: Vec<Transaction>,
    ) -> Self {
        let header = BlockHeader {
            height,
            previous_hash,
            timestamp: Timestamp::now(),
            proposer,
            transaction_root: Self::calculate_merkle_root(&transactions),
            state_root: [0u8; 32], // Would be actual state root in production
        };

        Self {
            header,
            transactions,
            signatures: HashMap::new(),
        }
    }

    /// Calculate the hash of this block
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        // Hash header fields
        hasher.update(&self.header.height.to_le_bytes());
        hasher.update(&self.header.previous_hash);
        hasher.update(&self.header.timestamp.as_secs().to_le_bytes());
        hasher.update(self.header.proposer.as_bytes());
        hasher.update(&self.header.transaction_root);
        hasher.update(&self.header.state_root);

        *hasher.finalize().as_bytes()
    }

    /// Calculate Merkle root of transactions
    fn calculate_merkle_root(transactions: &[Transaction]) -> [u8; 32] {
        if transactions.is_empty() {
            return [0u8; 32];
        }

        // Simple implementation: hash all transaction hashes together
        // Production would use proper Merkle tree
        let mut hasher = blake3::Hasher::new();

        for tx in transactions {
            hasher.update(&tx.hash());
        }

        *hasher.finalize().as_bytes()
    }

    /// Add a validator signature
    pub fn add_signature(&mut self, validator: NodeId, signature: Signature64) {
        self.signatures.insert(validator, signature);
    }

    /// Check if block has enough signatures for consensus (2f + 1)
    pub fn has_consensus(&self, required_signatures: usize) -> bool {
        self.signatures.len() >= required_signatures
    }

    /// Validate block structure and contents
    pub fn validate(&self, previous_block: Option<&Block>) -> Result<(), BlockError> {
        // Check height
        if let Some(prev) = previous_block {
            if self.header.height != prev.header.height + 1 {
                return Err(BlockError::InvalidHeight {
                    expected: prev.header.height + 1,
                    got: self.header.height,
                });
            }

            if self.header.previous_hash != prev.hash() {
                return Err(BlockError::InvalidPreviousHash);
            }
        } else if self.header.height != 0 {
            return Err(BlockError::InvalidGenesisHeight);
        }

        // Check timestamp is reasonable (not too far in future)
        let now = Timestamp::now();
        if self.header.timestamp.as_secs() > now.as_secs() + 60 {
            return Err(BlockError::FutureTimestamp);
        }

        // Verify transaction root
        let calculated_root = Self::calculate_merkle_root(&self.transactions);
        if self.header.transaction_root != calculated_root {
            return Err(BlockError::InvalidTransactionRoot);
        }

        // Check for duplicate transactions
        let mut tx_ids = std::collections::HashSet::new();
        for tx in &self.transactions {
            if !tx_ids.insert(tx.id) {
                return Err(BlockError::DuplicateTransaction(tx.id));
            }
        }

        Ok(())
    }

    /// Get block height
    pub fn height(&self) -> u64 {
        self.header.height
    }

    /// Get block timestamp
    pub fn timestamp(&self) -> Timestamp {
        self.header.timestamp
    }

    /// Get transaction count
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Check if block contains a transaction
    pub fn contains_transaction(&self, tx_id: &TransactionId) -> bool {
        self.transactions.iter().any(|tx| &tx.id == tx_id)
    }
}

/// Block header information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block height (0 for genesis)
    pub height: u64,

    /// Hash of previous block
    pub previous_hash: [u8; 32],

    /// When the block was created
    pub timestamp: Timestamp,

    /// Node that proposed this block
    pub proposer: NodeId,

    /// Merkle root of transactions
    pub transaction_root: [u8; 32],

    /// State root (account balances)
    pub state_root: [u8; 32],
}

/// Blockchain state
pub struct Blockchain {
    /// All blocks (height -> block)
    blocks: HashMap<u64, Block>,

    /// Current chain height
    height: u64,

    /// Genesis block hash
    genesis_hash: [u8; 32],
}

impl Blockchain {
    /// Create a new blockchain with genesis block
    pub fn new(genesis_block: Block) -> Self {
        let genesis_hash = genesis_block.hash();
        let mut blocks = HashMap::new();
        blocks.insert(0, genesis_block);

        Self {
            blocks,
            height: 0,
            genesis_hash,
        }
    }

    /// Add a block to the chain
    pub fn add_block(&mut self, block: Block) -> Result<(), BlockError> {
        // Validate against previous block
        let previous_block = self.blocks.get(&(block.height() - 1));
        block.validate(previous_block)?;

        // Add to chain
        self.blocks.insert(block.height(), block.clone());
        self.height = block.height();

        Ok(())
    }

    /// Get a block by height
    pub fn get_block(&self, height: u64) -> Option<&Block> {
        self.blocks.get(&height)
    }

    /// Get the latest block
    pub fn latest_block(&self) -> Option<&Block> {
        self.blocks.get(&self.height)
    }

    /// Get current height
    pub fn height(&self) -> u64 {
        self.height
    }

    /// Get genesis hash
    pub fn genesis_hash(&self) -> [u8; 32] {
        self.genesis_hash
    }

    /// Check if a transaction exists in the chain
    pub fn has_transaction(&self, tx_id: &TransactionId) -> bool {
        self.blocks
            .values()
            .any(|block| block.contains_transaction(tx_id))
    }

    /// Get all transactions in a range of blocks
    pub fn get_transactions(&self, from_height: u64, to_height: u64) -> Vec<Transaction> {
        let mut transactions = Vec::new();

        for height in from_height..=to_height {
            if let Some(block) = self.blocks.get(&height) {
                transactions.extend(block.transactions.clone());
            }
        }

        transactions
    }
}

/// Block validation errors
#[derive(Debug, thiserror::Error)]
pub enum BlockError {
    #[error("Invalid block height: expected {expected}, got {got}")]
    InvalidHeight { expected: u64, got: u64 },

    #[error("Invalid previous hash")]
    InvalidPreviousHash,

    #[error("Invalid genesis height (must be 0)")]
    InvalidGenesisHeight,

    #[error("Block timestamp is in the future")]
    FutureTimestamp,

    #[error("Invalid transaction root")]
    InvalidTransactionRoot,

    #[error("Duplicate transaction: {0}")]
    DuplicateTransaction(TransactionId),

    #[error("Insufficient signatures for consensus")]
    InsufficientSignatures,

    #[error("Transaction validation failed: {0}")]
    TransactionValidation(#[from] TransactionError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::transaction::TransactionType;
    use crate::identity::KeyPair;
    use anonnet_common::Credits;

    fn create_test_transaction(from: NodeId, to: NodeId, amount: u64) -> Transaction {
        Transaction::new(
            TransactionType::Transfer {
                from,
                to,
                amount: Credits::new(amount),
            },
            1,
        )
    }

    #[test]
    fn test_genesis_block_creation() {
        let keypair = KeyPair::generate();
        let proposer = NodeId::from_public_key(&keypair.public_key());

        let block = Block::new(0, [0u8; 32], proposer, vec![]);

        assert_eq!(block.height(), 0);
        assert_eq!(block.header.previous_hash, [0u8; 32]);
        assert_eq!(block.transaction_count(), 0);
    }

    #[test]
    fn test_block_hash() {
        let keypair = KeyPair::generate();
        let proposer = NodeId::from_public_key(&keypair.public_key());

        let block1 = Block::new(0, [0u8; 32], proposer, vec![]);
        let block2 = Block::new(0, [0u8; 32], proposer, vec![]);

        // Same content should produce same hash
        assert_eq!(block1.hash(), block2.hash());

        // Different content should produce different hash
        let block3 = Block::new(1, [0u8; 32], proposer, vec![]);
        assert_ne!(block1.hash(), block3.hash());
    }

    #[test]
    fn test_block_with_transactions() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let proposer = NodeId::from_public_key(&kp1.public_key());

        let from = NodeId::from_public_key(&kp1.public_key());
        let to = NodeId::from_public_key(&kp2.public_key());

        let tx = create_test_transaction(from, to, 100);
        let block = Block::new(0, [0u8; 32], proposer, vec![tx.clone()]);

        assert_eq!(block.transaction_count(), 1);
        assert!(block.contains_transaction(&tx.id));
    }

    #[test]
    fn test_block_signatures() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let proposer = NodeId::from_public_key(&kp1.public_key());
        let validator = NodeId::from_public_key(&kp2.public_key());

        let mut block = Block::new(0, [0u8; 32], proposer, vec![]);

        assert!(!block.has_consensus(2));

        block.add_signature(validator, Signature64([1u8; 64]));
        assert!(!block.has_consensus(2));

        block.add_signature(proposer, Signature64([2u8; 64]));
        assert!(block.has_consensus(2));
    }

    #[test]
    fn test_block_validation() {
        let keypair = KeyPair::generate();
        let proposer = NodeId::from_public_key(&keypair.public_key());

        // Valid genesis block
        let genesis = Block::new(0, [0u8; 32], proposer, vec![]);
        assert!(genesis.validate(None).is_ok());

        // Valid next block
        let genesis_hash = genesis.hash();
        let block1 = Block::new(1, genesis_hash, proposer, vec![]);
        assert!(block1.validate(Some(&genesis)).is_ok());

        // Invalid height
        let block_bad_height = Block::new(3, genesis_hash, proposer, vec![]);
        assert!(block_bad_height.validate(Some(&genesis)).is_err());

        // Invalid previous hash
        let block_bad_hash = Block::new(1, [0u8; 32], proposer, vec![]);
        assert!(block_bad_hash.validate(Some(&genesis)).is_err());
    }

    #[test]
    fn test_blockchain_creation() {
        let keypair = KeyPair::generate();
        let proposer = NodeId::from_public_key(&keypair.public_key());

        let genesis = Block::new(0, [0u8; 32], proposer, vec![]);
        let blockchain = Blockchain::new(genesis.clone());

        assert_eq!(blockchain.height(), 0);
        assert_eq!(blockchain.genesis_hash(), genesis.hash());
    }

    #[test]
    fn test_blockchain_add_block() {
        let keypair = KeyPair::generate();
        let proposer = NodeId::from_public_key(&keypair.public_key());

        let genesis = Block::new(0, [0u8; 32], proposer, vec![]);
        let mut blockchain = Blockchain::new(genesis.clone());

        let genesis_hash = genesis.hash();
        let block1 = Block::new(1, genesis_hash, proposer, vec![]);

        assert!(blockchain.add_block(block1).is_ok());
        assert_eq!(blockchain.height(), 1);
    }

    #[test]
    fn test_blockchain_transaction_lookup() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let proposer = NodeId::from_public_key(&kp1.public_key());

        let from = NodeId::from_public_key(&kp1.public_key());
        let to = NodeId::from_public_key(&kp2.public_key());

        let tx = create_test_transaction(from, to, 100);
        let tx_id = tx.id;

        let genesis = Block::new(0, [0u8; 32], proposer, vec![tx]);
        let blockchain = Blockchain::new(genesis);

        assert!(blockchain.has_transaction(&tx_id));
        assert!(!blockchain.has_transaction(&TransactionId::generate()));
    }

    #[test]
    fn test_merkle_root_calculation() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let proposer = NodeId::from_public_key(&kp1.public_key());

        let from = NodeId::from_public_key(&kp1.public_key());
        let to = NodeId::from_public_key(&kp2.public_key());

        let tx1 = create_test_transaction(from, to, 100);
        let tx2 = create_test_transaction(from, to, 200);

        let block = Block::new(0, [0u8; 32], proposer, vec![tx1, tx2]);

        // Validation should pass (merkle root is correct)
        assert!(block.validate(None).is_ok());
    }

    #[test]
    fn test_duplicate_transaction_detection() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let proposer = NodeId::from_public_key(&kp1.public_key());

        let from = NodeId::from_public_key(&kp1.public_key());
        let to = NodeId::from_public_key(&kp2.public_key());

        let tx = create_test_transaction(from, to, 100);

        let block = Block::new(0, [0u8; 32], proposer, vec![tx.clone(), tx.clone()]);

        // Should fail due to duplicate transaction
        assert!(block.validate(None).is_err());
    }
}
