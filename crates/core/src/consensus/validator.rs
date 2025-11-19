use crate::dht::RoutingTable;
use crate::identity::NodeId;
use anonnet_common::{Reputation, Timestamp};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashSet;

/// Validator selection and management
pub struct ValidatorSet {
    /// Active validators for current epoch
    validators: Vec<Validator>,

    /// Epoch number
    epoch: u64,

    /// When this epoch started
    epoch_start: Timestamp,

    /// Duration of each epoch (in seconds)
    epoch_duration: u64,
}

impl ValidatorSet {
    /// Create a new validator set
    pub fn new(epoch_duration: u64) -> Self {
        Self {
            validators: Vec::new(),
            epoch: 0,
            epoch_start: Timestamp::now(),
            epoch_duration,
        }
    }

    /// Select validators for the current epoch
    ///
    /// Uses reputation-weighted random selection to choose validators
    pub fn select_validators(
        &mut self,
        routing_table: &RoutingTable,
        count: usize,
    ) -> Result<Vec<Validator>, ValidatorError> {
        let candidates = Self::get_validator_candidates(routing_table);

        if candidates.is_empty() {
            return Err(ValidatorError::NoCandidates);
        }

        if candidates.len() < count {
            return Err(ValidatorError::InsufficientCandidates {
                available: candidates.len(),
                required: count,
            });
        }

        // Reputation-weighted selection
        let selected = Self::weighted_random_selection(&candidates, count)?;

        self.validators = selected
            .into_iter()
            .map(|(node_id, reputation)| Validator {
                node_id,
                reputation,
                votes_cast: 0,
                blocks_proposed: 0,
                joined_at: Timestamp::now(),
            })
            .collect();

        self.epoch += 1;
        self.epoch_start = Timestamp::now();

        Ok(self.validators.clone())
    }

    /// Get candidates eligible to be validators
    fn get_validator_candidates(routing_table: &RoutingTable) -> Vec<(NodeId, Reputation)> {
        routing_table
            .all_nodes()
            .into_iter()
            .filter(|entry| {
                // Must have high reputation
                if entry.reputation < Reputation::new(150) {
                    return false;
                }

                // Must not be stale
                if entry.is_stale() {
                    return false;
                }

                // Must accept relay (active participant)
                if !entry.accepts_relay {
                    return false;
                }

                true
            })
            .map(|entry| (entry.node_id, entry.reputation))
            .collect()
    }

    /// Weighted random selection based on reputation
    fn weighted_random_selection(
        candidates: &[(NodeId, Reputation)],
        count: usize,
    ) -> Result<Vec<(NodeId, Reputation)>, ValidatorError> {
        let mut rng = rand::thread_rng();
        let mut selected = Vec::new();
        let mut used = HashSet::new();

        // Calculate total weight
        let total_weight: u64 = candidates.iter().map(|(_, rep)| rep.score() as u64).sum();

        if total_weight == 0 {
            return Err(ValidatorError::InsufficientReputation);
        }

        // Select 'count' validators using weighted random selection
        while selected.len() < count && selected.len() < candidates.len() {
            let mut target = rng.gen_range(0..total_weight);
            let mut cumulative = 0u64;

            for (node_id, reputation) in candidates {
                if used.contains(node_id) {
                    continue;
                }

                cumulative += reputation.score() as u64;

                if cumulative > target {
                    selected.push((*node_id, *reputation));
                    used.insert(*node_id);
                    break;
                }
            }

            // Prevent infinite loop if we can't select enough
            if selected.len() == used.len() && used.len() == candidates.len() {
                break;
            }
        }

        if selected.len() < count.min(candidates.len()) {
            return Err(ValidatorError::SelectionFailed);
        }

        Ok(selected)
    }

    /// Check if it's time to rotate validators (new epoch)
    pub fn should_rotate(&self) -> bool {
        self.epoch_start.elapsed().as_secs() >= self.epoch_duration
    }

    /// Get current validators
    pub fn validators(&self) -> &[Validator] {
        &self.validators
    }

    /// Get validator count
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Check if a node is a validator
    pub fn is_validator(&self, node_id: &NodeId) -> bool {
        self.validators.iter().any(|v| &v.node_id == node_id)
    }

    /// Get validator by node ID
    pub fn get_validator(&self, node_id: &NodeId) -> Option<&Validator> {
        self.validators.iter().find(|v| &v.node_id == node_id)
    }

    /// Record a vote from a validator
    pub fn record_vote(&mut self, node_id: &NodeId) {
        if let Some(validator) = self.validators.iter_mut().find(|v| &v.node_id == node_id) {
            validator.votes_cast += 1;
        }
    }

    /// Record a block proposal from a validator
    pub fn record_proposal(&mut self, node_id: &NodeId) {
        if let Some(validator) = self.validators.iter_mut().find(|v| &v.node_id == node_id) {
            validator.blocks_proposed += 1;
        }
    }

    /// Calculate Byzantine fault tolerance threshold (2f + 1)
    ///
    /// Returns the minimum number of validators needed to reach consensus
    pub fn bft_threshold(&self) -> usize {
        let f = (self.validators.len() - 1) / 3; // Maximum faulty nodes
        2 * f + 1
    }

    /// Get current epoch
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

/// A validator in the consensus system
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validator {
    /// Node ID of the validator
    pub node_id: NodeId,

    /// Reputation score when selected
    pub reputation: Reputation,

    /// Number of votes cast in this epoch
    pub votes_cast: u64,

    /// Number of blocks proposed in this epoch
    pub blocks_proposed: u64,

    /// When the validator joined this epoch
    pub joined_at: Timestamp,
}

impl Validator {
    /// Check if validator is active (recently joined)
    pub fn is_active(&self) -> bool {
        self.joined_at.elapsed().as_secs() < 3600 // Active for 1 hour
    }
}

/// Validator selection errors
#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("No validator candidates available")]
    NoCandidates,

    #[error("Insufficient candidates: {available} < {required}")]
    InsufficientCandidates { available: usize, required: usize },

    #[error("Insufficient reputation among candidates")]
    InsufficientReputation,

    #[error("Validator selection failed")]
    SelectionFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;
    use anonnet_common::NetworkAddress;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn create_test_routing_table() -> RoutingTable {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let mut table = RoutingTable::new(local_id);

        // Add 30 test nodes with varying reputations
        for i in 0..30 {
            let keypair = KeyPair::generate();
            let node_id = NodeId::from_public_key(&keypair.public_key());
            let addr = NetworkAddress::from_socket(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, i as u8 + 1)),
                8080,
            ));

            table.insert(node_id, keypair.public_key(), vec![addr]).ok();

            // Set varying reputations (50 to 290)
            if let Some(entry) = table.find_node_mut(&node_id) {
                entry.reputation = Reputation::new((i * 10 + 50) as u32);
                entry.accepts_relay = true;
            }
        }

        table
    }

    #[test]
    fn test_validator_selection() {
        let table = create_test_routing_table();
        let mut validator_set = ValidatorSet::new(600); // 10 minute epochs

        let result = validator_set.select_validators(&table, 7);
        assert!(result.is_ok());

        let validators = result.unwrap();
        assert_eq!(validators.len(), 7);

        // All validators should have high reputation (>= 150)
        for validator in &validators {
            assert!(validator.reputation >= Reputation::new(150));
        }

        // All validators should be unique
        let unique_ids: HashSet<_> = validators.iter().map(|v| v.node_id).collect();
        assert_eq!(unique_ids.len(), validators.len());
    }

    #[test]
    fn test_validator_set_tracking() {
        let table = create_test_routing_table();
        let mut validator_set = ValidatorSet::new(600);

        validator_set.select_validators(&table, 5).unwrap();

        assert_eq!(validator_set.validator_count(), 5);
        assert_eq!(validator_set.epoch(), 1);

        // Check BFT threshold (2f + 1)
        let threshold = validator_set.bft_threshold();
        assert_eq!(threshold, 3); // With 5 validators, f=1, so 2*1+1=3
    }

    #[test]
    fn test_is_validator() {
        let table = create_test_routing_table();
        let mut validator_set = ValidatorSet::new(600);

        let validators = validator_set.select_validators(&table, 5).unwrap();

        assert!(validator_set.is_validator(&validators[0].node_id));
        assert!(!validator_set.is_validator(&NodeId::from_public_key(
            &KeyPair::generate().public_key()
        )));
    }

    #[test]
    fn test_record_vote() {
        let table = create_test_routing_table();
        let mut validator_set = ValidatorSet::new(600);

        let validators = validator_set.select_validators(&table, 5).unwrap();
        let validator_id = validators[0].node_id;

        validator_set.record_vote(&validator_id);
        validator_set.record_vote(&validator_id);

        let validator = validator_set.get_validator(&validator_id).unwrap();
        assert_eq!(validator.votes_cast, 2);
    }

    #[test]
    fn test_record_proposal() {
        let table = create_test_routing_table();
        let mut validator_set = ValidatorSet::new(600);

        let validators = validator_set.select_validators(&table, 5).unwrap();
        let validator_id = validators[0].node_id;

        validator_set.record_proposal(&validator_id);

        let validator = validator_set.get_validator(&validator_id).unwrap();
        assert_eq!(validator.blocks_proposed, 1);
    }

    #[test]
    fn test_insufficient_candidates() {
        let local_keypair = KeyPair::generate();
        let local_id = NodeId::from_public_key(&local_keypair.public_key());
        let table = RoutingTable::new(local_id); // Empty table

        let mut validator_set = ValidatorSet::new(600);
        let result = validator_set.select_validators(&table, 5);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidatorError::NoCandidates));
    }

    #[test]
    fn test_epoch_rotation() {
        let validator_set = ValidatorSet::new(10); // 10 second epochs

        // Should not need rotation immediately
        assert!(!validator_set.should_rotate());

        // After waiting, should need rotation
        std::thread::sleep(std::time::Duration::from_secs(11));
        assert!(validator_set.should_rotate());
    }

    #[test]
    fn test_weighted_selection_distribution() {
        let table = create_test_routing_table();
        let mut validator_set = ValidatorSet::new(600);

        // Run multiple selections and check that high-reputation nodes
        // appear more frequently
        let mut appearance_count = std::collections::HashMap::new();

        for _ in 0..100 {
            validator_set.select_validators(&table, 7).unwrap();
            for validator in validator_set.validators() {
                *appearance_count.entry(validator.node_id).or_insert(0) += 1;
            }
        }

        // At least some diversity in selection
        assert!(appearance_count.len() > 7);
    }
}
