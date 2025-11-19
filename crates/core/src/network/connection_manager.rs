/// Connection manager for handling peer connections
///
/// Manages the lifecycle of peer connections:
/// - Initiating connections with authentication handshake
/// - Accepting incoming connections and verifying peers
/// - Managing active connections
/// - Handling connection failures and reconnections

use crate::identity::{Identity, NodeId, PublicKey};
use crate::network::message_handler::{ConnectionHandler, MessageCodec};
use crate::protocol::messages::{
    ErrorCode, ErrorMessage, HandshakeMessage, HandshakeResponse, Message, MessagePayload,
    PeerInfo, Signature64,
};
use crate::transport::{Connection, Endpoint};
use anyhow::{anyhow, Result};
use anonnet_common::NetworkAddress;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Connection state for a peer
#[derive(Debug, Clone)]
pub struct PeerConnection {
    pub node_id: NodeId,
    pub public_key: PublicKey,
    pub handler: Arc<ConnectionHandler>,
    pub addresses: Vec<NetworkAddress>,
    pub accepts_relay: bool,
}

/// Connection manager
pub struct ConnectionManager {
    /// Our identity
    identity: Identity,

    /// QUIC endpoint
    endpoint: Arc<Endpoint>,

    /// Active peer connections
    connections: Arc<RwLock<HashMap<NodeId, PeerConnection>>>,

    /// Protocol version
    protocol_version: u32,

    /// Whether we accept relay traffic
    accepts_relay: bool,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(
        identity: Identity,
        endpoint: Arc<Endpoint>,
        accepts_relay: bool,
    ) -> Self {
        Self {
            identity,
            endpoint,
            connections: Arc::new(RwLock::new(HashMap::new())),
            protocol_version: 1,
            accepts_relay,
        }
    }

    /// Connect to a peer with authentication
    pub async fn connect_to_peer(
        &self,
        addr: SocketAddr,
    ) -> Result<Arc<ConnectionHandler>> {
        debug!("Connecting to peer at {}", addr);

        // Establish QUIC connection
        let connection = self.endpoint.connect(addr).await?;

        // Perform handshake
        let (peer_info, accepts_relay) = self.perform_handshake(&connection).await?;

        debug!("Successfully connected to peer: {}", peer_info.node_id);

        // Store connection
        let handler = Arc::new(ConnectionHandler::new(connection));
        let peer_conn = PeerConnection {
            node_id: peer_info.node_id,
            public_key: peer_info.public_key,
            handler: handler.clone(),
            addresses: peer_info.addresses,
            accepts_relay,
        };

        self.connections.write().await.insert(peer_info.node_id, peer_conn);

        Ok(handler)
    }

    /// Accept an incoming connection
    pub async fn accept_connection(&self) -> Result<Arc<ConnectionHandler>> {
        debug!("Waiting for incoming connection...");

        // Accept QUIC connection
        let connection = self.endpoint.accept().await?;
        let remote_addr = connection.remote_addr();

        debug!("Accepted connection from {}", remote_addr);

        // Wait for handshake from peer
        let (peer_info, accepts_relay) = self.receive_handshake(&connection).await?;

        info!("Peer {} connected from {}", peer_info.node_id, remote_addr);

        // Store connection
        let handler = Arc::new(ConnectionHandler::new(connection));
        let peer_conn = PeerConnection {
            node_id: peer_info.node_id,
            public_key: peer_info.public_key,
            handler: handler.clone(),
            addresses: peer_info.addresses,
            accepts_relay,
        };

        self.connections.write().await.insert(peer_info.node_id, peer_conn);

        Ok(handler)
    }

    /// Perform handshake as initiator
    async fn perform_handshake(&self, connection: &Connection) -> Result<(PeerInfo, bool)> {
        // Generate challenge nonce
        let mut nonce = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce);

        // Create handshake message
        let handshake = HandshakeMessage {
            node_id: self.identity.node_id(),
            public_key: self.identity.keypair().public_key(),
            protocol_version: self.protocol_version,
            addresses: vec![NetworkAddress::from_socket(self.endpoint.local_addr())],
            accepts_relay: self.accepts_relay,
            nonce,
        };

        let request = Message::new(MessagePayload::Handshake(handshake));

        // Send handshake and wait for response
        let response = MessageCodec::send_request(connection, request).await?;

        // Parse response
        match response.payload {
            MessagePayload::HandshakeResponse(resp) => {
                if !resp.success {
                    return Err(anyhow!("Handshake failed: {}", resp.error.unwrap_or_default()));
                }

                // Validate NodeID matches PublicKey
                let expected_node_id = NodeId::from_public_key(&resp.public_key);
                if resp.node_id != expected_node_id {
                    return Err(anyhow!("NodeID does not match PublicKey"));
                }

                // Verify challenge signature
                if !self.verify_challenge_signature(&resp.public_key, &nonce, &resp.challenge_signature) {
                    return Err(anyhow!("Challenge signature verification failed"));
                }

                debug!("Handshake successful with {}", resp.node_id);

                Ok((
                    PeerInfo {
                        node_id: resp.node_id,
                        public_key: resp.public_key,
                        addresses: resp.addresses,
                        last_seen: anonnet_common::Timestamp::now(),
                    },
                    resp.accepts_relay,
                ))
            }
            MessagePayload::Error(err) => {
                Err(anyhow!("Handshake error: {}", err.message))
            }
            _ => Err(anyhow!("Unexpected response type: {}", response.message_type())),
        }
    }

    /// Receive and respond to handshake as responder
    async fn receive_handshake(&self, connection: &Connection) -> Result<(PeerInfo, bool)> {
        // Accept incoming stream
        let (mut send, mut recv) = connection.accept_bi().await?;

        // Receive handshake
        let message = MessageCodec::recv_message(&mut recv).await?
            .ok_or_else(|| anyhow!("No handshake received"))?;

        match message.payload {
            MessagePayload::Handshake(handshake) => {
                // Validate NodeID matches PublicKey
                let expected_node_id = NodeId::from_public_key(&handshake.public_key);
                if handshake.node_id != expected_node_id {
                    let error_response = Message::new(MessagePayload::HandshakeResponse(
                        HandshakeResponse {
                            node_id: self.identity.node_id(),
                            public_key: self.identity.keypair().public_key(),
                            protocol_version: self.protocol_version,
                            addresses: vec![NetworkAddress::from_socket(self.endpoint.local_addr())],
                            accepts_relay: self.accepts_relay,
                            challenge_signature: Signature64([0u8; 64]),
                            success: false,
                            error: Some("NodeID does not match PublicKey".to_string()),
                        }
                    ));
                    MessageCodec::send_message_and_finish(send, &error_response).await?;
                    return Err(anyhow!("NodeID does not match PublicKey"));
                }

                // Sign the challenge
                let challenge_signature = self.sign_challenge(&handshake.nonce);

                // Send response
                let response = Message::new(MessagePayload::HandshakeResponse(
                    HandshakeResponse {
                        node_id: self.identity.node_id(),
                        public_key: self.identity.keypair().public_key(),
                        protocol_version: self.protocol_version,
                        addresses: vec![NetworkAddress::from_socket(self.endpoint.local_addr())],
                        accepts_relay: self.accepts_relay,
                        challenge_signature,
                        success: true,
                        error: None,
                    }
                ));

                MessageCodec::send_message_and_finish(send, &response).await?;

                debug!("Handshake completed with {}", handshake.node_id);

                Ok((
                    PeerInfo {
                        node_id: handshake.node_id,
                        public_key: handshake.public_key,
                        addresses: handshake.addresses,
                        last_seen: anonnet_common::Timestamp::now(),
                    },
                    handshake.accepts_relay,
                ))
            }
            _ => {
                let error_response = Message::new(MessagePayload::Error(ErrorMessage {
                    code: ErrorCode::InvalidMessage,
                    message: format!("Expected handshake, got {}", message.message_type()),
                }));
                MessageCodec::send_message_and_finish(send, &error_response).await?;
                Err(anyhow!("Expected handshake message"))
            }
        }
    }

    /// Sign a challenge nonce to prove private key ownership
    fn sign_challenge(&self, nonce: &[u8; 32]) -> Signature64 {
        let signature = self.identity.keypair().sign(nonce);
        Signature64(signature)
    }

    /// Verify a challenge signature
    fn verify_challenge_signature(
        &self,
        public_key: &PublicKey,
        nonce: &[u8; 32],
        signature: &Signature64,
    ) -> bool {
        public_key.verify(nonce, &signature.0)
    }

    /// Get a connection to a peer
    pub async fn get_connection(&self, node_id: &NodeId) -> Option<Arc<ConnectionHandler>> {
        self.connections.read().await.get(node_id).map(|c| c.handler.clone())
    }

    /// Get all connected peers
    pub async fn connected_peers(&self) -> Vec<NodeId> {
        self.connections.read().await.keys().copied().collect()
    }

    /// Disconnect from a peer
    pub async fn disconnect(&self, node_id: &NodeId) {
        if let Some(conn) = self.connections.write().await.remove(node_id) {
            conn.handler.connection().close(0, "disconnect");
            debug!("Disconnected from {}", node_id);
        }
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;
    use crate::transport::EndpointConfig;

    #[tokio::test]
    async fn test_connection_handshake() {
        // Create server
        let server_keypair = KeyPair::generate();
        let server_identity = Identity::from_keypair(server_keypair);
        let server_config = EndpointConfig::default();
        let server_endpoint = Arc::new(Endpoint::new(server_config).await.unwrap());
        let server_addr = server_endpoint.local_addr();

        let server_manager = Arc::new(ConnectionManager::new(
            server_identity,
            server_endpoint.clone(),
            true,
        ));

        // Create client
        let client_keypair = KeyPair::generate();
        let client_identity = Identity::from_keypair(client_keypair);
        let client_config = EndpointConfig::default();
        let client_endpoint = Arc::new(Endpoint::new(client_config).await.unwrap());

        let client_manager = ConnectionManager::new(
            client_identity,
            client_endpoint,
            false,
        );

        // Spawn server accept task
        let server_manager_clone = server_manager.clone();
        let accept_task = tokio::spawn(async move {
            server_manager_clone.accept_connection().await
        });

        // Client connects
        let client_handler = client_manager.connect_to_peer(server_addr).await.unwrap();

        // Server accepts
        let server_handler = accept_task.await.unwrap().unwrap();

        // Both should have established connection
        assert_eq!(client_manager.connection_count().await, 1);
        assert_eq!(server_manager.connection_count().await, 1);
    }
}
