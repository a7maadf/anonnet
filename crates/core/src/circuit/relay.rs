use super::crypto::{CryptoError, OnionCrypto};
use super::types::{Circuit, RelayCell, RelayCellType};
use crate::identity::NodeId;

/// Relay handler for processing cells
pub struct RelayHandler;

impl RelayHandler {
    /// Process an incoming relay cell
    ///
    /// This function is called by a relay node to process a cell
    pub fn process_cell(
        circuit: &mut Circuit,
        cell: RelayCell,
        our_position: usize,
    ) -> Result<RelayAction, RelayError> {
        // Verify digest
        if !cell.verify_digest() {
            return Err(RelayError::InvalidDigest);
        }

        // Check if we're at the right position
        if our_position >= circuit.length() {
            return Err(RelayError::InvalidPosition);
        }

        // Determine action based on cell type
        match cell.cell_type {
            RelayCellType::Data => {
                // Data cell - update stats and forward
                circuit.add_received(cell.payload.len() as u64);
                Ok(RelayAction::Forward {
                    next_hop: our_position + 1,
                    cell,
                })
            }

            RelayCellType::Begin => {
                // Begin a new stream
                Ok(RelayAction::BeginStream {
                    stream_id: cell.stream_id,
                })
            }

            RelayCellType::End => {
                // End a stream
                Ok(RelayAction::EndStream {
                    stream_id: cell.stream_id,
                })
            }

            RelayCellType::Extend => {
                // Extend the circuit (only at the last hop)
                if our_position == circuit.length() - 1 {
                    Ok(RelayAction::ExtendCircuit { cell })
                } else {
                    Err(RelayError::InvalidExtend)
                }
            }

            RelayCellType::Extended => {
                // Circuit extended successfully
                Ok(RelayAction::CircuitExtended { cell })
            }

            RelayCellType::Truncate => {
                // Truncate circuit at this point
                Ok(RelayAction::TruncateCircuit {
                    at_position: our_position,
                })
            }

            RelayCellType::Truncated => {
                // Circuit was truncated
                Ok(RelayAction::CircuitTruncated)
            }

            RelayCellType::Sendme => {
                // Flow control acknowledgment
                Ok(RelayAction::Acknowledge {
                    stream_id: cell.stream_id,
                })
            }

            RelayCellType::Drop => {
                // Drop this cell (for padding/cover traffic)
                Ok(RelayAction::Drop)
            }
        }
    }

    /// Forward a cell to the next hop
    pub fn forward_cell(
        circuit: &Circuit,
        cell: &RelayCell,
        from_hop: usize,
    ) -> Result<Option<NodeId>, RelayError> {
        let next_hop = from_hop + 1;

        if next_hop >= circuit.length() {
            // We're the exit node - deliver to destination
            return Ok(None);
        }

        // Get the next node
        let next_node = &circuit.nodes[next_hop];
        Ok(Some(next_node.node_id))
    }

    /// Encrypt a cell for sending through a circuit
    pub fn encrypt_cell_for_circuit(
        circuit: &Circuit,
        cell: &RelayCell,
    ) -> Result<Vec<u8>, CryptoError> {
        // Serialize the cell
        let serialized = bincode::serialize(cell)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // Encrypt with onion layers
        OnionCrypto::encrypt_onion(circuit, &serialized)
    }

    /// Decrypt a cell received at a hop
    pub fn decrypt_cell_at_hop(
        encrypted: &[u8],
        hop_key: &[u8; 32],
    ) -> Result<RelayCell, CryptoError> {
        // Decrypt one layer
        let decrypted = OnionCrypto::decrypt_layer(hop_key, encrypted)?;

        // Deserialize
        bincode::deserialize(&decrypted)
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Create a data cell
    pub fn create_data_cell(stream_id: u16, data: Vec<u8>, sequence: u32) -> RelayCell {
        let mut cell = RelayCell::new(RelayCellType::Data, stream_id, data);
        cell.sequence = sequence;
        cell.set_digest();
        cell
    }

    /// Create a begin stream cell
    pub fn create_begin_cell(stream_id: u16, target: Vec<u8>) -> RelayCell {
        let mut cell = RelayCell::new(RelayCellType::Begin, stream_id, target);
        cell.set_digest();
        cell
    }

    /// Create an end stream cell
    pub fn create_end_cell(stream_id: u16, reason: u8) -> RelayCell {
        let mut cell = RelayCell::new(RelayCellType::End, stream_id, vec![reason]);
        cell.set_digest();
        cell
    }
}

/// Actions to take when processing a relay cell
#[derive(Debug)]
pub enum RelayAction {
    /// Forward the cell to the next hop
    Forward { next_hop: usize, cell: RelayCell },

    /// Begin a new stream
    BeginStream { stream_id: u16 },

    /// End a stream
    EndStream { stream_id: u16 },

    /// Extend the circuit
    ExtendCircuit { cell: RelayCell },

    /// Circuit was extended
    CircuitExtended { cell: RelayCell },

    /// Truncate circuit at position
    TruncateCircuit { at_position: usize },

    /// Circuit was truncated
    CircuitTruncated,

    /// Acknowledge data (flow control)
    Acknowledge { stream_id: u16 },

    /// Drop this cell
    Drop,
}

/// Errors in relay processing
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error("Invalid digest in relay cell")]
    InvalidDigest,

    #[error("Invalid position in circuit")]
    InvalidPosition,

    #[error("Invalid extend command (not at exit)")]
    InvalidExtend,

    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Serialization error")]
    Serialization,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::types::{CircuitId, CircuitNode, CircuitPurpose};
    use crate::identity::KeyPair;

    fn create_test_circuit() -> Circuit {
        let id = CircuitId::generate();
        let mut circuit = Circuit::new(id, CircuitPurpose::General);

        for _ in 0..3 {
            let keypair = KeyPair::generate();
            let node_id = NodeId::from_public_key(&keypair.public_key());
            let enc_key = OnionCrypto::generate_key();
            let dec_key = OnionCrypto::generate_key();

            let node = CircuitNode::new(node_id, keypair.public_key(), enc_key, dec_key);
            circuit.add_node(node);
        }

        circuit
    }

    #[test]
    fn test_create_data_cell() {
        let cell = RelayHandler::create_data_cell(1, vec![1, 2, 3], 0);

        assert_eq!(cell.cell_type, RelayCellType::Data);
        assert_eq!(cell.stream_id, 1);
        assert_eq!(cell.payload, vec![1, 2, 3]);
        assert!(cell.verify_digest());
    }

    #[test]
    fn test_process_data_cell() {
        let mut circuit = create_test_circuit();
        let cell = RelayHandler::create_data_cell(1, vec![1, 2, 3, 4, 5], 0);

        let result = RelayHandler::process_cell(&mut circuit, cell, 0);
        assert!(result.is_ok());

        if let Ok(RelayAction::Forward { next_hop, .. }) = result {
            assert_eq!(next_hop, 1);
        } else {
            panic!("Expected Forward action");
        }
    }

    #[test]
    fn test_forward_cell() {
        let circuit = create_test_circuit();
        let cell = RelayHandler::create_data_cell(1, vec![1, 2, 3], 0);

        let result = RelayHandler::forward_cell(&circuit, &cell, 0);
        assert!(result.is_ok());

        let next_node = result.unwrap();
        assert!(next_node.is_some());
    }

    #[test]
    fn test_forward_at_exit() {
        let circuit = create_test_circuit();
        let cell = RelayHandler::create_data_cell(1, vec![1, 2, 3], 0);

        // At the last hop (exit node)
        let result = RelayHandler::forward_cell(&circuit, &cell, 2);
        assert!(result.is_ok());

        let next_node = result.unwrap();
        assert!(next_node.is_none()); // No next hop - we're at exit
    }
}
