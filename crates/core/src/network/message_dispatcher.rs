/// Message dispatcher for routing protocol messages to appropriate handlers
///
/// This module provides the central dispatch mechanism for all protocol messages:
/// - DHT operations (FindNode, NodesFound)
/// - Circuit management (CreateCircuit, RelayData, etc.)
/// - Health checks (Ping, Pong)
/// - Credit system messages

use crate::dht::DHT;
use crate::identity::NodeId;
use crate::network::ConnectionHandler;
use crate::protocol::messages::{
    ErrorCode, ErrorMessage, FindNodeMessage, FindValueMessage, Message, MessagePayload,
    NodesFoundMessage, PeerInfo, PingMessage, PongMessage, Signature64, StoreMessage,
    StoreResponseMessage, StoredValueMessage, ValueFoundMessage,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Message dispatcher
pub struct MessageDispatcher {
    /// DHT reference
    dht: Arc<RwLock<DHT>>,
}

impl MessageDispatcher {
    /// Create a new message dispatcher
    pub fn new(dht: Arc<RwLock<DHT>>) -> Self {
        Self { dht }
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

            // Circuit messages (stub for now)
            MessagePayload::CreateCircuit(_) => {
                warn!("CreateCircuit not yet implemented");
                Ok(Some(Message::new(MessagePayload::Error(ErrorMessage {
                    code: ErrorCode::InternalError,
                    message: "Circuit creation not yet implemented".to_string(),
                }))))
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
