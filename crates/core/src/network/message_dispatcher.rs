/// Message dispatcher for routing protocol messages to appropriate handlers
///
/// This module provides the central dispatch mechanism for all protocol messages:
/// - DHT operations (FindNode, NodesFound)
/// - Circuit management (CreateCircuit, RelayData, etc.)
/// - Health checks (Ping, Pong)
/// - Credit system messages

use crate::circuit::{handle_create_circuit, CircuitManager, RelayHandler};
use crate::dht::DHT;
use crate::identity::NodeId;
use crate::network::{ConnectionHandler, ConnectionManager};
use crate::protocol::messages::{
    CircuitCreatedMessage, CreateCircuitMessage, ErrorCode, ErrorMessage, FindNodeMessage,
    FindValueMessage, Message, MessagePayload, NodesFoundMessage, PeerInfo, PingMessage,
    PongMessage, RelayCellMessage, Signature64, StoreMessage, StoreResponseMessage, StoredValueMessage,
    ValueFoundMessage,
};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Message dispatcher
pub struct MessageDispatcher {
    /// DHT reference
    dht: Arc<RwLock<DHT>>,

    /// Circuit manager for relay cell processing (with interior mutability)
    circuit_manager: RwLock<Option<Arc<RwLock<CircuitManager>>>>,

    /// Connection manager for forwarding (with interior mutability)
    connection_manager: RwLock<Option<Arc<ConnectionManager>>>,

    /// Our node ID
    node_id: NodeId,
}

impl MessageDispatcher {
    /// Create a new message dispatcher
    pub fn new(dht: Arc<RwLock<DHT>>) -> Self {
        let node_id = {
            let runtime = tokio::runtime::Handle::try_current()
                .expect("MessageDispatcher must be created from within a tokio runtime");
            runtime.block_on(async {
                dht.read().await.local_id()
            })
        };
        Self {
            dht,
            circuit_manager: RwLock::new(None),
            connection_manager: RwLock::new(None),
            node_id,
        }
    }

    /// Set the circuit manager (called after initialization)
    pub async fn set_circuit_manager(&self, circuit_manager: Arc<RwLock<CircuitManager>>) {
        *self.circuit_manager.write().await = Some(circuit_manager);
    }

    /// Set the connection manager (called after initialization)
    pub async fn set_connection_manager(&self, connection_manager: Arc<ConnectionManager>) {
        *self.connection_manager.write().await = Some(connection_manager);
    }

    /// Dispatch an incoming message and optionally return a response
    pub async fn dispatch(&self, message: Message) -> Result<Option<Message>> {
        debug!("Dispatching {} message", message.message_type());

        match message.payload {
            // Peer discovery messages
            MessagePayload::FindNode(find_node) => {
                self.handle_find_node(find_node).await
            }
            MessagePayload::NodesFound(nodes_found) => {
                self.handle_nodes_found(nodes_found).await
            }

            // DHT storage messages
            MessagePayload::Store(store) => {
                self.handle_store(store).await
            }
            MessagePayload::FindValue(find_value) => {
                self.handle_find_value(find_value).await
            }
            MessagePayload::StoreResponse(_) | MessagePayload::ValueFound(_) => {
                // These are responses, no reply needed
                Ok(None)
            }

            // Health checks
            MessagePayload::Ping(ping) => {
                Ok(Some(self.handle_ping(ping)))
            }
            MessagePayload::Pong(_pong) => {
                // Pong is a response, no reply needed
                Ok(None)
            }

            // Circuit messages
            MessagePayload::CreateCircuit(create) => {
                self.handle_create_circuit(create).await
            }
            MessagePayload::CircuitCreated(_) | MessagePayload::CircuitFailed(_) => {
                // These are responses, no reply needed
                Ok(None)
            }

            // Relay cell forwarding
            MessagePayload::RelayCell(relay_cell) => {
                self.handle_relay_cell(relay_cell).await
            }

            // Other messages
            _ => {
                warn!("Unhandled message type: {}", message.message_type());
                Ok(None)
            }
        }
    }

    /// Handle FindNode request
    async fn handle_find_node(&self, find_node: FindNodeMessage) -> Result<Option<Message>> {
        debug!("FindNode request for target: {}", find_node.target);

        let dht = self.dht.read().await;
        let closest = dht.closest_nodes(&find_node.target, find_node.count);

        // Convert to PeerInfo
        let peers: Vec<PeerInfo> = closest
            .iter()
            .map(|entry| PeerInfo {
                node_id: entry.node_id,
                public_key: entry.public_key,
                addresses: entry.addresses.clone(),
                last_seen: entry.last_seen,
            })
            .collect();

        info!("Returning {} nodes for FindNode query", peers.len());

        Ok(Some(Message::new(MessagePayload::NodesFound(
            NodesFoundMessage { nodes: peers },
        ))))
    }

    /// Handle NodesFound response
    async fn handle_nodes_found(&self, nodes_found: NodesFoundMessage) -> Result<Option<Message>> {
        debug!("Received {} nodes in NodesFound", nodes_found.nodes.len());

        let mut dht = self.dht.write().await;

        // Add discovered nodes to routing table
        for peer in nodes_found.nodes {
            if let Err(e) = dht.add_node(peer.node_id, peer.public_key, peer.addresses) {
                debug!("Failed to add node {}: {}", peer.node_id, e);
            }
        }

        // NodesFound is a response, no reply needed
        Ok(None)
    }

    /// Handle Ping request
    fn handle_ping(&self, ping: PingMessage) -> Message {
        debug!("Ping request with nonce: {}", ping.nonce);

        Message::new(MessagePayload::Pong(PongMessage {
            nonce: ping.nonce,
        }))
    }

    /// Handle Store request
    async fn handle_store(&self, store: StoreMessage) -> Result<Option<Message>> {
        use crate::dht::StoredValue;
        use std::time::Duration;

        debug!("Store request for key: {:?}", &store.key[..8]);

        let mut dht = self.dht.write().await;

        // Create stored value
        let value = StoredValue::new(store.value, store.publisher)
            .with_ttl(Duration::from_secs(store.ttl));

        // Store in DHT
        match dht.store(store.key, value) {
            Ok(()) => {
                info!("Successfully stored value");
                Ok(Some(Message::new(MessagePayload::StoreResponse(
                    StoreResponseMessage {
                        success: true,
                        error: None,
                    },
                ))))
            }
            Err(e) => {
                warn!("Failed to store value: {}", e);
                Ok(Some(Message::new(MessagePayload::StoreResponse(
                    StoreResponseMessage {
                        success: false,
                        error: Some(e.to_string()),
                    },
                ))))
            }
        }
    }

    /// Handle FindValue request
    async fn handle_find_value(&self, find_value: FindValueMessage) -> Result<Option<Message>> {
        debug!("FindValue request for key: {:?}", &find_value.key[..8]);

        let dht = self.dht.read().await;

        // Try to find value in local storage
        if let Some(values) = dht.find_value(&find_value.key) {
            info!("Found {} values for key", values.len());

            // Convert to message format
            let value_messages: Vec<StoredValueMessage> = values
                .iter()
                .map(|v| StoredValueMessage {
                    data: v.data.clone(),
                    publisher: v.publisher,
                    stored_at: v.stored_at.as_secs(),
                    ttl: v.ttl.as_secs(),
                    signature: v.signature.as_ref().map(|s| Signature64(
                        s.as_slice().try_into().unwrap_or([0u8; 64])
                    )),
                })
                .collect();

            Ok(Some(Message::new(MessagePayload::ValueFound(
                ValueFoundMessage {
                    found: true,
                    values: value_messages,
                    closest_nodes: vec![],
                },
            ))))
        } else {
            // Value not found, return closest nodes
            info!("Value not found, returning closest nodes");

            // Convert key to NodeId for distance calculation
            let target = NodeId::from_bytes(find_value.key);
            let closest = dht.closest_nodes(&target, 20);

            let peers: Vec<PeerInfo> = closest
                .iter()
                .map(|entry| PeerInfo {
                    node_id: entry.node_id,
                    public_key: entry.public_key,
                    addresses: entry.addresses.clone(),
                    last_seen: entry.last_seen,
                })
                .collect();

            Ok(Some(Message::new(MessagePayload::ValueFound(
                ValueFoundMessage {
                    found: false,
                    values: vec![],
                    closest_nodes: peers,
                },
            ))))
        }
    }

    /// Handle CreateCircuit request (for relays)
    async fn handle_create_circuit(
        &self,
        create: CreateCircuitMessage,
    ) -> Result<Option<Message>> {
        let circuit_id = create.circuit_id;
        debug!("Handling CreateCircuit request for circuit {:?}", circuit_id);

        match handle_create_circuit(create, self.node_id).await {
            Ok(response) => {
                Ok(Some(Message::new(MessagePayload::CircuitCreated(response))))
            }
            Err(e) => {
                warn!("Failed to create circuit: {}", e);
                Ok(Some(Message::new(MessagePayload::CircuitFailed(
                    crate::protocol::messages::CircuitFailedMessage {
                        circuit_id,
                        reason: e.to_string(),
                    },
                ))))
            }
        }
    }

    /// Handle RelayCell messages (forward encrypted cells through circuit)
    async fn handle_relay_cell(
        &self,
        relay_cell: crate::protocol::messages::RelayCellMessage,
    ) -> Result<Option<Message>> {
        debug!("Handling RelayCell for circuit {:?}", relay_cell.circuit_id);

        // Get circuit manager
        let circuit_manager = self.circuit_manager.read().await.clone()
            .ok_or_else(|| anyhow!("Circuit manager not initialized"))?;

        let connection_manager = self.connection_manager.read().await.clone()
            .ok_or_else(|| anyhow!("Connection manager not initialized"))?;

        // Convert protocol CircuitId to internal CircuitId
        let circuit_id = crate::circuit::CircuitId(relay_cell.circuit_id.0);

        // Look up the circuit
        let mut manager = circuit_manager.write().await;
        let circuit = manager.get_circuit_mut(&circuit_id)
            .ok_or_else(|| anyhow!("Circuit {} not found", circuit_id))?;

        // Determine our position in the circuit
        // Try to decrypt one layer - find which hop we are
        let mut decrypted_cell = None;
        let mut our_hop_index = None;

        for (i, node) in circuit.nodes.iter_mut().enumerate() {
            if node.node_id == self.node_id {
                // This is us! Decrypt with our backward_crypto
                match RelayHandler::decrypt_cell_at_hop(
                    &relay_cell.encrypted_payload,
                    &mut node.backward_crypto,
                ) {
                    Ok(cell) => {
                        decrypted_cell = Some(cell);
                        our_hop_index = Some(i);
                        break;
                    }
                    Err(e) => {
                        debug!("Failed to decrypt at hop {}: {}", i, e);
                        continue;
                    }
                }
            }
        }

        let cell = decrypted_cell.ok_or_else(|| {
            anyhow!("Failed to decrypt relay cell (not a hop in this circuit or decryption failed)")
        })?;

        let hop_index = our_hop_index.unwrap();

        debug!("Decrypted relay cell at hop {}: {:?}", hop_index, cell.cell_type);

        // Determine if we're the exit node or need to forward
        if hop_index == circuit.nodes.len() - 1 {
            // We're the exit node - process the cell
            debug!("Processing relay cell as exit node");

            // TODO: Handle different cell types:
            // - BEGIN: establish connection to destination
            // - DATA: forward to destination
            // - END: close connection
            // For now, just log it
            debug!("Exit node processing not yet implemented for {:?}", cell.cell_type);

            Ok(None)
        } else {
            // We're a relay node - forward to next hop
            let next_hop_index = hop_index + 1;
            let next_node = &circuit.nodes[next_hop_index];
            let next_node_id = next_node.node_id;

            debug!("Forwarding relay cell to hop {} ({})", next_hop_index, next_node_id);

            // Re-serialize and forward
            // Note: In a full onion routing implementation, we would re-encrypt
            // with all remaining layers. For now, we forward the decrypted cell.
            let next_handler = connection_manager.get_connection(&next_node_id)
                .ok_or_else(|| anyhow!("No connection to next hop {}", next_node_id))?;

            // Forward the cell
            let forward_msg = Message::new(MessagePayload::RelayCell(RelayCellMessage {
                circuit_id: relay_cell.circuit_id,
                encrypted_payload: bincode::serialize(&cell)
                    .map_err(|e| anyhow!("Failed to serialize cell: {}", e))?,
            }));

            // Send as one-way message
            next_handler.send_message(forward_msg).await?;

            debug!("Relay cell forwarded to hop {}", next_hop_index);
            Ok(None)
        }
    }

    /// Perform a DHT lookup by querying nodes
    pub async fn perform_lookup(
        &self,
        target: NodeId,
        handler: Arc<ConnectionHandler>,
    ) -> Result<Vec<PeerInfo>> {
        debug!("Starting DHT lookup for {}", target);

        // Get nodes to query from DHT
        let nodes_to_query = {
            let mut dht = self.dht.write().await;
            dht.start_lookup(target);
            dht.next_lookup_queries(&target).unwrap_or_default()
        };

        if nodes_to_query.is_empty() {
            warn!("No nodes to query for lookup");
            return Ok(vec![]);
        }

        debug!("Querying {} nodes for lookup", nodes_to_query.len());

        // Send FindNode requests
        let find_node = Message::new(MessagePayload::FindNode(FindNodeMessage {
            target,
            count: 20,
        }));

        let response = handler.send_request(find_node).await?;

        // Process response
        match response.payload {
            MessagePayload::NodesFound(nodes_found) => {
                debug!("Lookup found {} nodes", nodes_found.nodes.len());

                // Add to DHT
                let mut dht = self.dht.write().await;
                for peer in &nodes_found.nodes {
                    dht.add_node(peer.node_id, peer.public_key, peer.addresses.clone()).ok();
                }

                Ok(nodes_found.nodes)
            }
            MessagePayload::Error(err) => {
                Err(anyhow::anyhow!("Lookup failed: {}", err.message))
            }
            _ => {
                Err(anyhow::anyhow!("Unexpected response type"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[tokio::test]
    async fn test_handle_ping() {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());
        let dht = Arc::new(RwLock::new(DHT::new(node_id, vec![])));

        let dispatcher = MessageDispatcher::new(dht);

        let ping = PingMessage { nonce: 12345 };
        let response = dispatcher.handle_ping(ping);

        match response.payload {
            MessagePayload::Pong(pong) => assert_eq!(pong.nonce, 12345),
            _ => panic!("Expected Pong response"),
        }
    }

    #[tokio::test]
    async fn test_handle_find_node() {
        let keypair = KeyPair::generate();
        let node_id = NodeId::from_public_key(&keypair.public_key());
        let dht = Arc::new(RwLock::new(DHT::new(node_id, vec![])));

        // Add some nodes to DHT
        {
            let mut dht = dht.write().await;
            for _ in 0..5 {
                let kp = KeyPair::generate();
                let nid = NodeId::from_public_key(&kp.public_key());
                let pk = kp.public_key();
                let addr = anonnet_common::NetworkAddress::from_socket(
                    "127.0.0.1:8080".parse().unwrap(),
                );
                dht.add_node(nid, pk, vec![addr]).ok();
            }
        }

        let dispatcher = MessageDispatcher::new(dht);

        let target_kp = KeyPair::generate();
        let target = NodeId::from_public_key(&target_kp.public_key());

        let find_node = FindNodeMessage {
            target,
            count: 3,
        };

        let response = dispatcher.handle_find_node(find_node).await.unwrap().unwrap();

        match response.payload {
            MessagePayload::NodesFound(nodes) => {
                assert!(nodes.nodes.len() <= 3);
            }
            _ => panic!("Expected NodesFound response"),
        }
    }
}
