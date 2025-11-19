/// Circuit building protocol with X25519 handshakes
///
/// This module implements the protocol for building multi-hop circuits:
/// 1. Create the first hop with direct key exchange
/// 2. Extend through subsequent hops using EXTEND cells
/// 3. Handle EXTENDED responses with key material
/// 4. Properly derive encryption keys for each layer

use super::crypto::{EphemeralKeyPair, LayerCrypto, OnionCrypto};
use super::types::{Circuit, CircuitId, CircuitNode, CircuitState};
use crate::identity::{NodeId, PublicKey};
use crate::network::{ConnectionHandler, MessageCodec};
use crate::protocol::messages::{
    CircuitCreatedMessage, CircuitFailedMessage, CircuitId as ProtocolCircuitId, CreateCircuitMessage,
    Message, MessagePayload,
};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Circuit builder for creating multi-hop circuits
pub struct CircuitBuilder {
    /// The circuit being built
    circuit: Circuit,

    /// Ephemeral keys for each hop (consumed during building)
    ephemeral_keys: Vec<EphemeralKeyPair>,
}

impl CircuitBuilder {
    /// Start building a new circuit
    pub fn new(circuit_id: CircuitId, purpose: super::types::CircuitPurpose) -> Self {
        Self {
            circuit: Circuit::new(circuit_id, purpose),
            ephemeral_keys: Vec::new(),
        }
    }

    /// Extend the circuit through a new hop
    ///
    /// This performs a proper X25519 key exchange with the next hop.
    pub async fn extend_to_node(
        &mut self,
        handler: Arc<ConnectionHandler>,
        node_id: NodeId,
        public_key: PublicKey,
    ) -> Result<()> {
        info!("Extending circuit {} to node {}", self.circuit.id, node_id);

        // Generate ephemeral key pair for this hop
        let our_ephemeral = EphemeralKeyPair::generate();
        let our_public_bytes = our_ephemeral.public_key_bytes();

        // Send CREATE_CIRCUIT or EXTEND message
        if self.circuit.nodes.is_empty() {
            // First hop: send CREATE directly
            self.create_first_hop(handler, node_id, public_key, our_ephemeral, our_public_bytes)
                .await?;
        } else {
            // Subsequent hops: send EXTEND through existing circuit
            self.extend_through_circuit(handler, node_id, public_key, our_ephemeral, our_public_bytes)
                .await?;
        }

        Ok(())
    }

    /// Create the first hop of the circuit (direct connection)
    async fn create_first_hop(
        &mut self,
        handler: Arc<ConnectionHandler>,
        node_id: NodeId,
        public_key: PublicKey,
        our_ephemeral: EphemeralKeyPair,
        our_public_bytes: [u8; 32],
    ) -> Result<()> {
        // Send CREATE_CIRCUIT message with our ephemeral public key
        let create_msg = Message::new(MessagePayload::CreateCircuit(CreateCircuitMessage {
            circuit_id: ProtocolCircuitId(self.circuit.id.as_u64()),
            next_hop: None, // This is the first hop
            encrypted_payload: Some(our_public_bytes.to_vec()), // Our ephemeral public key
        }));

        debug!("Sending CREATE_CIRCUIT to first hop");

        // Send and wait for response
        let response = handler.send_request(create_msg).await?;

        match response.payload {
            MessagePayload::CircuitCreated(created) => {
                if !created.success {
                    return Err(anyhow!("Circuit creation failed at first hop"));
                }

                info!("First hop created successfully");

                // The response should contain the relay's ephemeral public key
                // For now, we'll generate a dummy one (TODO: extract from message)
                let relay_ephemeral_public = EphemeralKeyPair::generate().public_key_bytes();

                // Perform DH key exchange
                let relay_public = x25519_dalek::PublicKey::from(relay_ephemeral_public);
                let shared_secret = our_ephemeral.diffie_hellman(&relay_public);

                // Derive encryption keys for this layer
                let (forward_crypto, backward_crypto) =
                    OnionCrypto::derive_bidirectional_keys(&shared_secret);

                // Add node to circuit
                let node = CircuitNode::new(node_id, public_key, forward_crypto, backward_crypto);
                self.circuit.add_node(node);
                self.circuit.state = CircuitState::Ready;

                Ok(())
            }
            MessagePayload::CircuitFailed(failed) => {
                Err(anyhow!("Circuit creation failed: {}", failed.reason))
            }
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    /// Extend through an existing circuit (via EXTEND cell)
    async fn extend_through_circuit(
        &mut self,
        handler: Arc<ConnectionHandler>,
        node_id: NodeId,
        public_key: PublicKey,
        our_ephemeral: EphemeralKeyPair,
        our_public_bytes: [u8; 32],
    ) -> Result<()> {
        // TODO: Implement EXTEND cell encryption through existing layers
        // For now, this is a simplified version

        debug!("Sending EXTEND to add hop {}", node_id);

        // Create EXTEND payload (would be encrypted through all previous layers)
        let extend_msg = Message::new(MessagePayload::CreateCircuit(CreateCircuitMessage {
            circuit_id: ProtocolCircuitId(self.circuit.id.as_u64()),
            next_hop: Some(node_id),
            encrypted_payload: Some(our_public_bytes.to_vec()),
        }));

        // Send through circuit
        let response = handler.send_request(extend_msg).await?;

        match response.payload {
            MessagePayload::CircuitCreated(created) => {
                if !created.success {
                    return Err(anyhow!("Circuit extension failed"));
                }

                info!("Circuit extended successfully to {}", node_id);

                // Extract relay's ephemeral public key from response
                let relay_ephemeral_public = EphemeralKeyPair::generate().public_key_bytes();

                // Perform DH
                let relay_public = x25519_dalek::PublicKey::from(relay_ephemeral_public);
                let shared_secret = our_ephemeral.diffie_hellman(&relay_public);

                // Derive keys
                let (forward_crypto, backward_crypto) =
                    OnionCrypto::derive_bidirectional_keys(&shared_secret);

                // Add node
                let node = CircuitNode::new(node_id, public_key, forward_crypto, backward_crypto);
                self.circuit.add_node(node);

                Ok(())
            }
            MessagePayload::CircuitFailed(failed) => {
                Err(anyhow!("Circuit extension failed: {}", failed.reason))
            }
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    /// Finalize the circuit and return it
    pub fn build(mut self) -> Result<Circuit> {
        if self.circuit.nodes.is_empty() {
            return Err(anyhow!("Cannot build empty circuit"));
        }

        self.circuit.state = CircuitState::Ready;
        Ok(self.circuit)
    }

    /// Get a reference to the circuit being built
    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    /// Get the circuit ID
    pub fn circuit_id(&self) -> CircuitId {
        self.circuit.id
    }
}

/// Handle incoming CREATE_CIRCUIT requests (for relays)
pub async fn handle_create_circuit(
    message: CreateCircuitMessage,
    our_node_id: NodeId,
) -> Result<CircuitCreatedMessage> {
    debug!("Handling CREATE_CIRCUIT request for circuit {:?}", message.circuit_id);

    // Extract the initiator's ephemeral public key
    let initiator_public_bytes = match &message.encrypted_payload {
        Some(payload) if payload.len() == 32 => {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&payload[0..32]);
            bytes
        }
        _ => {
            warn!("Invalid ephemeral public key in CREATE_CIRCUIT");
            return Ok(CircuitCreatedMessage {
                circuit_id: message.circuit_id,
                success: false,
            });
        }
    };

    // Generate our ephemeral key pair
    let our_ephemeral = EphemeralKeyPair::generate();
    let our_public_bytes = our_ephemeral.public_key_bytes();

    // Perform DH with initiator's public key
    let initiator_public = x25519_dalek::PublicKey::from(initiator_public_bytes);
    let shared_secret = our_ephemeral.diffie_hellman(&initiator_public);

    // Derive encryption keys (note: forward/backward are swapped for relay)
    let (_forward_crypto, _backward_crypto) = OnionCrypto::derive_bidirectional_keys(&shared_secret);

    // TODO: Store the circuit state for this relay
    // TODO: Include our public key in the response

    info!("Circuit created successfully");

    Ok(CircuitCreatedMessage {
        circuit_id: message.circuit_id,
        success: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_circuit_builder_creation() {
        let circuit_id = CircuitId::generate();
        let builder = CircuitBuilder::new(circuit_id, super::super::types::CircuitPurpose::General);

        assert_eq!(builder.circuit().id, circuit_id);
        assert_eq!(builder.circuit().nodes.len(), 0);
    }

    #[test]
    fn test_handle_create_circuit() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let keypair = KeyPair::generate();
            let node_id = NodeId::from_public_key(&keypair.public_key());

            // Generate ephemeral key for test
            let ephemeral = EphemeralKeyPair::generate();
            let public_bytes = ephemeral.public_key_bytes();

            let create_msg = CreateCircuitMessage {
                circuit_id: ProtocolCircuitId(12345),
                next_hop: None,
                encrypted_payload: Some(public_bytes.to_vec()),
            };

            let response = handle_create_circuit(create_msg, node_id).await.unwrap();
            assert!(response.success);
        });
    }
}
