use super::block::{Block, Blockchain};
use super::transaction::{Transaction, TransactionError, TransactionId, TransactionType};
use crate::identity::{NodeId, PublicKey};
use crate::protocol::Signature64;
use anonnet_common::Credits;
use std::collections::{HashMap, HashSet};

/// Credit ledger that tracks account balances and processes transactions
pub struct CreditLedger {
    /// Account balances (node_id -> credits)
    balances: HashMap<NodeId, Credits>,

    /// Processed transaction IDs (for replay protection)
    processed_transactions: HashSet<TransactionId>,

    /// Transaction nonces (node_id -> nonce)
    nonces: HashMap<NodeId, u64>,

    /// Blockchain reference
    blockchain: Blockchain,
}

impl CreditLedger {
    /// Create a new credit ledger with genesis block
    pub fn new(genesis_block: Block) -> Self {
        let blockchain = Blockchain::new(genesis_block.clone());

        let mut ledger = Self {
            balances: HashMap::new(),
            processed_transactions: HashSet::new(),
            nonces: HashMap::new(),
            blockchain,
        };

        // Process genesis transactions
        for tx in genesis_block.transactions {
            // Genesis transactions don't need full validation
            let _ = ledger.process_genesis_transaction(tx);
        }

        ledger
    }

    /// Process a genesis transaction (relaxed validation)
    fn process_genesis_transaction(&mut self, tx: Transaction) -> Result<(), TransactionError> {
        match &tx.tx_type {
            TransactionType::Genesis { recipient, amount } => {
                let balance = self.get_balance(recipient);
                self.balances.insert(*recipient, balance + *amount);
                self.processed_transactions.insert(tx.id);
                self.nonces.insert(tx.sender(), tx.nonce);
                Ok(())
            }
            _ => Err(TransactionError::InvalidAmount(0)), // Only genesis txs allowed
        }
    }

    /// Get balance for a node
    pub fn get_balance(&self, node_id: &NodeId) -> Credits {
        self.balances.get(node_id).copied().unwrap_or(Credits::new(0))
    }

    /// Set balance for a node (for testing/genesis)
    pub fn set_balance(&mut self, node_id: NodeId, balance: Credits) {
        self.balances.insert(node_id, balance);
    }

    /// Validate a transaction
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<(), TransactionError> {
        // Check for replay attacks
        if self.processed_transactions.contains(&tx.id) {
            return Err(TransactionError::ReplayAttack);
        }

        // Check timestamp (not too old)
        if tx.timestamp.elapsed().as_secs() > 3600 {
            return Err(TransactionError::Expired);
        }

        // Check nonce
        let sender = tx.sender();
        let expected_nonce = self.nonces.get(&sender).copied().unwrap_or(0) + 1;
        if tx.nonce != expected_nonce {
            return Err(TransactionError::ReplayAttack);
        }

        // Validate based on transaction type
        match &tx.tx_type {
            TransactionType::Transfer { from, to, amount } => {
                // Check sender matches
                if from != &sender {
                    return Err(TransactionError::InvalidSender);
                }

                // Check balance
                let balance = self.get_balance(from);
                if balance < *amount {
                    return Err(TransactionError::InsufficientBalance {
                        needed: amount.amount(),
                        available: balance.amount(),
                    });
                }

                // Check amount is non-zero
                if amount.amount() == 0 {
                    return Err(TransactionError::InvalidAmount(0));
                }
            }

            TransactionType::RelayReward {
                relay_node,
                proof,
                amount,
                ..
            } => {
                // Verify the relay proof
                if !proof.verify() {
                    return Err(TransactionError::InvalidRelayProof);
                }

                // Verify the amount matches the proof
                let calculated_credits = proof.calculate_credits();
                if amount != &calculated_credits {
                    return Err(TransactionError::InvalidAmount(amount.amount()));
                }
            }

            TransactionType::Genesis { amount, .. } => {
                // Genesis transactions can only be in genesis block
                if self.blockchain.height() > 0 {
                    return Err(TransactionError::InvalidAmount(amount.amount()));
                }
            }
        }

        Ok(())
    }

    /// Process a transaction and update balances
    pub fn process_transaction(&mut self, tx: Transaction) -> Result<(), TransactionError> {
        // Validate first
        self.validate_transaction(&tx)?;

        // Process based on type
        match &tx.tx_type {
            TransactionType::Transfer { from, to, amount } => {
                let from_balance = self.get_balance(from);
                let to_balance = self.get_balance(to);

                self.balances.insert(*from, from_balance - *amount);
                self.balances.insert(*to, to_balance + *amount);
            }

            TransactionType::RelayReward {
                relay_node, amount, ..
            } => {
                // Reward the relay node
                let balance = self.get_balance(relay_node);
                self.balances.insert(*relay_node, balance + *amount);
            }

            TransactionType::Genesis { recipient, amount } => {
                // Initial credit distribution
                let balance = self.get_balance(recipient);
                self.balances.insert(*recipient, balance + *amount);
            }
        }

        // Record transaction as processed
        self.processed_transactions.insert(tx.id);

        // Update nonce
        let sender = tx.sender();
        self.nonces.insert(sender, tx.nonce);

        Ok(())
    }

    /// Process a block of transactions
    pub fn process_block(&mut self, block: Block) -> Result<(), TransactionError> {
        // Add block to blockchain
        self.blockchain
            .add_block(block.clone())
            .map_err(|_| TransactionError::InvalidAmount(0))?;

        // Process all transactions
        for tx in block.transactions {
            self.process_transaction(tx)?;
        }

        Ok(())
    }

    /// Get blockchain reference
    pub fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }

    /// Get total supply of credits
    pub fn total_supply(&self) -> Credits {
        self.balances.values().fold(Credits::new(0), |acc, &c| acc + c)
    }

    /// Get number of accounts
    pub fn account_count(&self) -> usize {
        self.balances.len()
    }

    /// Check if a transaction has been processed
    pub fn is_processed(&self, tx_id: &TransactionId) -> bool {
        self.processed_transactions.contains(tx_id)
    }

    /// Get the next nonce for a node
    pub fn next_nonce(&self, node_id: &NodeId) -> u64 {
        self.nonces.get(node_id).copied().unwrap_or(0) + 1
    }

    /// Export ledger state (for debugging/testing)
    pub fn export_balances(&self) -> HashMap<NodeId, Credits> {
        self.balances.clone()
    }
}

/// Transaction validator with signature verification
pub struct TransactionValidator {
    /// Known public keys (node_id -> public_key)
    public_keys: HashMap<NodeId, PublicKey>,
}

impl TransactionValidator {
    pub fn new() -> Self {
        Self {
            public_keys: HashMap::new(),
        }
    }

    /// Register a public key for a node
    pub fn register_key(&mut self, node_id: NodeId, public_key: PublicKey) {
        self.public_keys.insert(node_id, public_key);
    }

    /// Verify transaction signature
    pub fn verify_signature(&self, tx: &Transaction) -> Result<(), TransactionError> {
        let sender = tx.sender();

        // Get public key
        let public_key = self
            .public_keys
            .get(&sender)
            .ok_or(TransactionError::InvalidSender)?;

        // Verify signature
        let hash = tx.hash();

        // In production, use proper signature verification
        // For now, just check signature is not all zeros
        if tx.signature == Signature64([0u8; 64]) {
            return Err(TransactionError::InvalidSignature);
        }

        // Would verify: public_key.verify(&hash, &tx.signature)

        Ok(())
    }

    /// Validate and verify a transaction
    pub fn validate_and_verify(
        &self,
        ledger: &CreditLedger,
        tx: &Transaction,
    ) -> Result<(), TransactionError> {
        // Validate transaction logic
        ledger.validate_transaction(tx)?;

        // Verify signature
        self.verify_signature(tx)?;

        Ok(())
    }
}

impl Default for TransactionValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::transaction::RelayProof;
    use crate::identity::KeyPair;

    fn create_genesis_block(recipient: NodeId, amount: u64) -> Block {
        let proposer = recipient;

        let tx = Transaction::new(
            TransactionType::Genesis {
                recipient,
                amount: Credits::new(amount),
            },
            1,
        );

        Block::new(0, [0u8; 32], proposer, vec![tx])
    }

    #[test]
    fn test_ledger_creation() {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        let genesis = create_genesis_block(node_id, 1000);
        let ledger = CreditLedger::new(genesis);

        // Genesis transactions are processed automatically
        assert_eq!(ledger.get_balance(&node_id), Credits::new(1000));
    }

    #[test]
    fn test_transfer_transaction() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        let node1 = NodeId::from_public_key(&kp1.public_key());
        let node2 = NodeId::from_public_key(&kp2.public_key());

        // Create ledger with initial balance
        let genesis = create_genesis_block(node1, 1000);
        let mut ledger = CreditLedger::new(genesis);

        // Create transfer
        let tx = Transaction::new(
            TransactionType::Transfer {
                from: node1,
                to: node2,
                amount: Credits::new(300),
            },
            2, // nonce = 2 (genesis was nonce 1)
        );

        ledger.process_transaction(tx).unwrap();

        assert_eq!(ledger.get_balance(&node1), Credits::new(700));
        assert_eq!(ledger.get_balance(&node2), Credits::new(300));
    }

    #[test]
    fn test_insufficient_balance() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        let node1 = NodeId::from_public_key(&kp1.public_key());
        let node2 = NodeId::from_public_key(&kp2.public_key());

        let genesis = create_genesis_block(node1, 100);
        let mut ledger = CreditLedger::new(genesis);

        // Try to transfer more than balance
        let tx = Transaction::new(
            TransactionType::Transfer {
                from: node1,
                to: node2,
                amount: Credits::new(500),
            },
            2,
        );

        let result = ledger.process_transaction(tx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransactionError::InsufficientBalance { .. }
        ));
    }

    #[test]
    fn test_relay_reward() {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        let genesis = create_genesis_block(node_id, 0);
        let mut ledger = CreditLedger::new(genesis);

        // Create relay proof
        let proof = RelayProof::new(12345, 100, 1024 * 1024 * 1024); // 1 GB
        let amount = proof.calculate_credits();

        let tx = Transaction::new(
            TransactionType::RelayReward {
                relay_node: node_id,
                circuit_id: 12345,
                bytes_relayed: 1024 * 1024 * 1024,
                amount,
                proof,
            },
            2,
        );

        ledger.process_transaction(tx).unwrap();

        // Should have earned 1000 credits for 1 GB
        assert_eq!(ledger.get_balance(&node_id), Credits::new(1000));
    }

    #[test]
    fn test_replay_attack_prevention() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        let node1 = NodeId::from_public_key(&kp1.public_key());
        let node2 = NodeId::from_public_key(&kp2.public_key());

        let genesis = create_genesis_block(node1, 1000);
        let mut ledger = CreditLedger::new(genesis);

        let tx = Transaction::new(
            TransactionType::Transfer {
                from: node1,
                to: node2,
                amount: Credits::new(100),
            },
            2,
        );

        // First time should succeed
        ledger.process_transaction(tx.clone()).unwrap();

        // Second time should fail (replay attack)
        let result = ledger.process_transaction(tx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransactionError::ReplayAttack
        ));
    }

    #[test]
    fn test_nonce_tracking() {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        let genesis = create_genesis_block(node_id, 1000);
        let mut ledger = CreditLedger::new(genesis);

        assert_eq!(ledger.next_nonce(&node_id), 2);

        let tx = Transaction::new(
            TransactionType::Transfer {
                from: node_id,
                to: node_id,
                amount: Credits::new(1),
            },
            2,
        );

        ledger.process_transaction(tx).unwrap();

        assert_eq!(ledger.next_nonce(&node_id), 3);
    }

    #[test]
    fn test_total_supply() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();

        let node1 = NodeId::from_public_key(&kp1.public_key());
        let node2 = NodeId::from_public_key(&kp2.public_key());

        let genesis = create_genesis_block(node1, 1000);
        let mut ledger = CreditLedger::new(genesis);

        assert_eq!(ledger.total_supply(), Credits::new(1000));

        // Transfer doesn't change total supply
        let tx = Transaction::new(
            TransactionType::Transfer {
                from: node1,
                to: node2,
                amount: Credits::new(300),
            },
            2,
        );

        ledger.process_transaction(tx).unwrap();

        assert_eq!(ledger.total_supply(), Credits::new(1000));
    }

    #[test]
    fn test_transaction_validator() {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());

        let mut validator = TransactionValidator::new();
        validator.register_key(node_id, keypair.public_key());

        let tx = Transaction::new(
            TransactionType::Genesis {
                recipient: node_id,
                amount: Credits::new(1000),
            },
            1,
        )
        .with_signature(Signature64([1u8; 64])); // Non-zero signature

        // Should fail because we don't do proper signature verification yet
        // In production, this would actually verify the signature
        assert!(validator.verify_signature(&tx).is_ok());
    }
}
