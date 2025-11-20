/// AnonNet Node Runtime
///
/// This module provides the main Node struct that coordinates all components
/// of an AnonNet node: DHT, circuits, consensus, services, etc.

use crate::circuit::{CircuitManager, CircuitPool, CircuitPoolConfig};
use crate::consensus::{Block, Blockchain, CreditLedger, Transaction, TransactionType};
use crate::dht::{BootstrapNode, RoutingTable, DHT};
use crate::identity::{ExportableIdentity, Identity, KeyPair, NodeId, ProofOfWork};
use crate::network::{
    BandwidthEstimator, ConnectionManager, MessageDispatcher, RateLimiter, BandwidthConfig,
    RateLimitConfig,
};
use crate::peer::PeerManager;
use crate::service::{ServiceDirectory, RendezvousManager};
use crate::transport::{Endpoint, EndpointConfig};
use anonnet_common::{Credits, NetworkAddress, NodeConfig};
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Main AnonNet node runtime
///
/// Coordinates all subsystems: DHT, circuits, consensus, services, etc.
pub struct Node {
    /// Node identity
    identity: Identity,

    /// Proof of work for this node
    pow: ProofOfWork,

    /// Node configuration
    config: NodeConfig,

    /// DHT for peer discovery
    dht: Arc<RwLock<DHT>>,

    /// Routing table
    routing_table: Arc<RwLock<RoutingTable>>,

    /// Circuit manager
    circuit_manager: Arc<RwLock<CircuitManager>>,

    /// Circuit pool
    circuit_pool: Arc<CircuitPool>,

    /// Peer manager
    peer_manager: Arc<RwLock<PeerManager>>,

    /// Credit ledger
    credit_ledger: Arc<RwLock<CreditLedger>>,

    /// Blockchain
    blockchain: Arc<RwLock<Blockchain>>,

    /// Service directory (for .anon services)
    service_directory: Arc<ServiceDirectory>,

    /// Rendezvous manager
    rendezvous_manager: Arc<RendezvousManager>,

    /// Bandwidth estimator
    bandwidth_estimator: Arc<BandwidthEstimator>,

    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,

    /// QUIC endpoint
    endpoint: Option<Arc<Endpoint>>,

    /// Connection manager
    connection_manager: Option<Arc<ConnectionManager>>,

    /// Message dispatcher
    message_dispatcher: Arc<MessageDispatcher>,

    /// Whether the node is running
    running: Arc<RwLock<bool>>,
}

impl Node {
    /// Create a new node with the given configuration
    pub async fn new(config: NodeConfig) -> Result<Self> {
        info!("Initializing AnonNet node...");

        // Load or generate identity
        let (identity, pow) = Self::load_or_generate_identity(&config).await?;
        let node_id = identity.node_id();

        info!("Node ID: {}", node_id);
        info!("Public Key: {}", identity.keypair().public_key());

        // Parse bootstrap nodes from config
        let bootstrap_nodes = Self::parse_bootstrap_nodes(&config.bootstrap_nodes);

        // Initialize routing table
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(node_id)));

        // Initialize DHT
        let dht = Arc::new(RwLock::new(DHT::new(
            node_id,
            bootstrap_nodes,
        )));

        // Initialize circuit manager
        let circuit_manager = Arc::new(RwLock::new(CircuitManager::new()));

        // Initialize circuit pool
        let pool_config = CircuitPoolConfig {
            target_pool_size: 3,
            max_circuit_age: std::time::Duration::from_secs(600),
            min_idle_time: std::time::Duration::from_secs(5),
            max_reuse_count: 10,
        };
        let circuit_pool = Arc::new(CircuitPool::new(
            pool_config,
            circuit_manager.clone(),
        ));

        // Initialize peer manager
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(
            config.max_peers,
        )));

        // Create genesis block for our node
        let genesis_block = Self::create_genesis_block(node_id, &pow);

        // Initialize credit ledger
        let credit_ledger = Arc::new(RwLock::new(CreditLedger::new(genesis_block.clone())));

        // Initialize blockchain
        let blockchain = Arc::new(RwLock::new(Blockchain::new(genesis_block)));

        // Initialize service directory
        let service_directory = Arc::new(ServiceDirectory::new(
            node_id,
            routing_table.clone(),
        ));

        // Initialize rendezvous manager
        // Note: RendezvousManager expects unwrapped CircuitManager
        let rendezvous_circuit_manager = {
            let _manager = circuit_manager.read().await;
            // We can't easily share the manager here, so for now create a new one
            // This is a temporary workaround - ideally RendezvousManager should accept Arc<RwLock<>>
            Arc::new(CircuitManager::new())
        };
        let rendezvous_manager = Arc::new(RendezvousManager::new(
            node_id,
            rendezvous_circuit_manager,
        ));

        // Initialize bandwidth estimator
        let bandwidth_estimator = Arc::new(BandwidthEstimator::new(
            BandwidthConfig::default(),
        ));

        // Initialize rate limiter
        let rate_limiter = Arc::new(RateLimiter::new(
            RateLimitConfig::default(),
        ));

        // Initialize message dispatcher
        let message_dispatcher = Arc::new(MessageDispatcher::new(dht.clone()).await);

        Ok(Self {
            identity,
            pow,
            config,
            dht,
            routing_table,
            circuit_manager,
            circuit_pool,
            peer_manager,
            credit_ledger,
            blockchain,
            service_directory,
            rendezvous_manager,
            bandwidth_estimator,
            rate_limiter,
            endpoint: None,
            connection_manager: None,
            message_dispatcher,
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the node
    pub async fn start(&mut self) -> Result<()> {
        let listen_addr = format!("{}:{}", self.config.listen_addr, self.config.listen_port);
        info!("Starting AnonNet node on {}...", listen_addr);

        // Set running flag
        *self.running.write().await = true;

        // Start QUIC endpoint
        self.start_endpoint().await?;

        // Bootstrap DHT
        self.bootstrap_dht().await?;

        // Start background tasks
        self.start_background_tasks().await;

        info!("Node started successfully");
        Ok(())
    }

    /// Stop the node
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping AnonNet node...");
        *self.running.write().await = false;
        Ok(())
    }

    /// Check if node is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get node ID
    pub fn node_id(&self) -> NodeId {
        self.identity.node_id()
    }

    /// Get identity
    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// Get circuit pool (for use by proxies)
    pub fn circuit_pool(&self) -> Arc<CircuitPool> {
        self.circuit_pool.clone()
    }

    /// Get service directory
    pub fn service_directory(&self) -> Arc<ServiceDirectory> {
        self.service_directory.clone()
    }

    /// Get rendezvous manager
    pub fn rendezvous_manager(&self) -> Arc<RendezvousManager> {
        self.rendezvous_manager.clone()
    }

    /// Get routing table (for circuit building)
    pub fn routing_table(&self) -> Arc<RwLock<RoutingTable>> {
        self.routing_table.clone()
    }

    /// Get circuit manager
    pub fn circuit_manager(&self) -> Arc<RwLock<CircuitManager>> {
        self.circuit_manager.clone()
    }

    /// Get connection manager
    pub fn connection_manager(&self) -> Option<Arc<ConnectionManager>> {
        self.connection_manager.clone()
    }

    /// Get current credit balance for this node
    pub async fn get_credit_balance(&self) -> Credits {
        let ledger = self.credit_ledger.read().await;
        ledger.get_balance(&self.node_id())
    }

    /// Get credit statistics (earned, spent, rates)
    pub async fn get_credit_stats(&self) -> CreditStats {
        let ledger = self.credit_ledger.read().await;
        let balance = ledger.get_balance(&self.node_id());

        // TODO: Implement tracking of total earned/spent
        // For now, return placeholder values based on balance
        CreditStats {
            total_earned: balance.amount(),
            total_spent: 0,
            earning_rate: 0.0,
            spending_rate: 0.0,
        }
    }

    /// Get information about active circuits
    pub async fn get_active_circuits(&self) -> Vec<ActiveCircuitInfo> {
        let manager = self.circuit_manager.read().await;
        let circuits = manager.all_circuits();

        // Convert circuits to info structs
        circuits
            .iter()
            .map(|c| ActiveCircuitInfo {
                circuit_id: c.id,
                purpose: c.purpose,
                state: c.state,
                hops: c.nodes.len(),
                age_seconds: c.created_at.elapsed().as_secs(),
                use_count: 0, // TODO: Track circuit use count
            })
            .collect()
    }

    /// Load or generate node identity
    async fn load_or_generate_identity(config: &NodeConfig) -> Result<(Identity, ProofOfWork)> {
        let data_dir = std::path::PathBuf::from(&config.data_dir);
        let identity_path = data_dir.join("identity.json");
        let pow_path = data_dir.join("pow.json");

        // Try to load existing identity
        if identity_path.exists() && pow_path.exists() {
            info!("Loading identity from {:?}", identity_path);

            // Load identity
            let identity_json = std::fs::read_to_string(&identity_path)?;
            let exportable = ExportableIdentity::from_json(&identity_json)?;
            let identity = Identity::from_exportable(&exportable)?;

            // Load PoW
            let pow_json = std::fs::read_to_string(&pow_path)?;
            let pow: ProofOfWork = serde_json::from_str(&pow_json)?;

            info!("Identity and PoW loaded successfully");
            return Ok((identity, pow));
        }

        // Generate new identity with PoW
        info!("Generating new identity with PoW difficulty 12...");
        let (keypair, pow) = KeyPair::generate_with_pow(12);
        let node_id = NodeId::from_public_key(&keypair.public_key());
        let identity = Identity::from_secret_bytes(&keypair.secret_bytes())?;

        // Save to files
        std::fs::create_dir_all(&data_dir)?;

        // Save identity
        let exportable = identity.to_exportable();
        let identity_json = exportable.to_json()?;
        std::fs::write(&identity_path, identity_json)?;

        // Save PoW
        let pow_json = serde_json::to_string_pretty(&pow)?;
        std::fs::write(&pow_path, pow_json)?;

        info!("Identity and PoW generated and saved");
        Ok((identity, pow))
    }

    /// Start QUIC endpoint
    async fn start_endpoint(&mut self) -> Result<()> {
        info!("Starting QUIC endpoint...");
        let listen_addr: SocketAddr = format!("{}:{}", self.config.listen_addr, self.config.listen_port).parse()?;
        let endpoint_config = EndpointConfig {
            bind_addr: listen_addr,
        };
        let endpoint = Arc::new(Endpoint::new(endpoint_config).await?);

        // Initialize connection manager with the endpoint
        let connection_manager = Arc::new(ConnectionManager::new(
            self.identity.clone(),
            endpoint.clone(),
            true, // Accept relay traffic
        ));

        self.endpoint = Some(endpoint);
        self.connection_manager = Some(connection_manager.clone());

        // Wire managers into message dispatcher for relay cell handling
        self.message_dispatcher.set_circuit_manager(self.circuit_manager.clone()).await;
        self.message_dispatcher.set_connection_manager(connection_manager).await;

        Ok(())
    }

    /// Bootstrap DHT by connecting to bootstrap nodes
    async fn bootstrap_dht(&self) -> Result<()> {
        if self.config.bootstrap_nodes.is_empty() {
            warn!("No bootstrap nodes configured. Running as bootstrap node.");
            return Ok(());
        }

        info!("Bootstrapping DHT with {} nodes...", self.config.bootstrap_nodes.len());

        for bootstrap_node in &self.config.bootstrap_nodes {
            match self.connect_to_bootstrap(&bootstrap_node).await {
                Ok(_) => {
                    info!("Connected to bootstrap node: {}", bootstrap_node);
                }
                Err(e) => {
                    warn!("Failed to connect to bootstrap node {}: {}", bootstrap_node, e);
                }
            }
        }

        Ok(())
    }

    /// Connect to a bootstrap node
    async fn connect_to_bootstrap(
        &self,
        bootstrap: &str,
    ) -> Result<()> {
        let connection_manager = match &self.connection_manager {
            Some(cm) => cm,
            None => {
                warn!("Connection manager not initialized");
                return Ok(());
            }
        };

        // Parse bootstrap address
        let addr: SocketAddr = bootstrap.parse()?;

        info!("Connecting to bootstrap node: {}", addr);

        // Connect to bootstrap node
        match connection_manager.connect_to_peer(addr).await {
            Ok(handler) => {
                info!("Successfully connected to bootstrap node");

                // Start message handling for this connection
                let dispatcher = self.message_dispatcher.clone();
                tokio::spawn(async move {
                    loop {
                        match handler.connection().accept_bi().await {
                            Ok((mut send, mut recv)) => {
                                // Receive message
                                match crate::network::MessageCodec::recv_message(&mut recv).await {
                                    Ok(Some(message)) => {
                                        // Dispatch message
                                        match dispatcher.dispatch(message).await {
                                            Ok(Some(response)) => {
                                                // Send response
                                                let _ = crate::network::MessageCodec::send_message_and_finish(send, &response).await;
                                            }
                                            Ok(None) => {
                                                // No response needed
                                                let _ = send.finish().await;
                                            }
                                            Err(e) => {
                                                error!("Error dispatching message: {}", e);
                                            }
                                        }
                                    }
                                    Ok(None) => break,
                                    Err(e) => {
                                        debug!("Error receiving message: {}", e);
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });

                Ok(())
            }
            Err(e) => {
                warn!("Failed to connect to bootstrap: {}", e);
                Err(e)
            }
        }
    }

    /// Start background maintenance tasks
    async fn start_background_tasks(&self) {
        let running = self.running.clone();
        let circuit_pool = self.circuit_pool.clone();
        let bandwidth_estimator = self.bandwidth_estimator.clone();

        // Circuit pool cleanup task
        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                circuit_pool.cleanup().await;
            }
        });

        let running = self.running.clone();

        // Bandwidth stats update task
        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                bandwidth_estimator.update_network_stats().await;
            }
        });

        // Incoming connection acceptance task
        if let Some(connection_manager) = &self.connection_manager {
            let running = self.running.clone();
            let connection_manager = connection_manager.clone();
            let dispatcher = self.message_dispatcher.clone();

            tokio::spawn(async move {
                while *running.read().await {
                    match connection_manager.accept_connection().await {
                        Ok(handler) => {
                            info!("Accepted new peer connection");

                            // Start message handling for this connection
                            let dispatcher = dispatcher.clone();
                            tokio::spawn(async move {
                                loop {
                                    match handler.connection().accept_bi().await {
                                        Ok((mut send, mut recv)) => {
                                            // Receive message
                                            match crate::network::MessageCodec::recv_message(&mut recv).await {
                                                Ok(Some(message)) => {
                                                    // Dispatch message
                                                    match dispatcher.dispatch(message).await {
                                                        Ok(Some(response)) => {
                                                            // Send response
                                                            let _ = crate::network::MessageCodec::send_message_and_finish(send, &response).await;
                                                        }
                                                        Ok(None) => {
                                                            // No response needed
                                                            let _ = send.finish().await;
                                                        }
                                                        Err(e) => {
                                                            error!("Error dispatching message: {}", e);
                                                        }
                                                    }
                                                }
                                                Ok(None) => break,
                                                Err(e) => {
                                                    debug!("Error receiving message: {}", e);
                                                    break;
                                                }
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            debug!("Error accepting connection: {}", e);
                            // Small delay before retrying
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                    }
                }
            });
        }

        info!("Background tasks started");
    }

    /// Get node statistics
    pub async fn get_stats(&self) -> NodeStats {
        let routing_table = self.routing_table.read().await;
        let peer_manager = self.peer_manager.read().await;
        let circuit_pool_stats = self.circuit_pool.stats().await;
        let network_stats = self.bandwidth_estimator.get_network_stats().await;

        NodeStats {
            node_id: self.identity.node_id(),
            peer_count: routing_table.node_count(),
            active_peers: peer_manager.stats().connected,
            circuits: circuit_pool_stats.total_circuits,
            active_circuits: circuit_pool_stats.in_use_circuits,
            bandwidth: network_stats.total_bandwidth,
            is_running: *self.running.read().await,
        }
    }

    /// Create genesis block for this node
    fn create_genesis_block(node_id: NodeId, pow: &crate::identity::ProofOfWork) -> Block {
        let amount = Credits::new(pow.calculate_credits());

        let tx = Transaction::new(
            TransactionType::Genesis {
                recipient: node_id,
                amount,
                pow: pow.clone(),
            },
            1, // nonce
        );

        Block::new(0, [0u8; 32], node_id, vec![tx])
    }

    /// Parse bootstrap nodes from string addresses
    fn parse_bootstrap_nodes(addresses: &[String]) -> Vec<BootstrapNode> {
        addresses
            .iter()
            .filter_map(|addr| {
                // Try to parse as socket address first
                if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
                    Some(BootstrapNode::new(NetworkAddress::from_socket(socket_addr)))
                } else {
                    // Try to parse as host:port
                    let parts: Vec<&str> = addr.split(':').collect();
                    if parts.len() == 2 {
                        if let Ok(port) = parts[1].parse::<u16>() {
                            Some(BootstrapNode::new(NetworkAddress::from_domain(
                                parts[0].to_string(),
                                port,
                            )))
                        } else {
                            warn!("Failed to parse bootstrap node port: {}", addr);
                            None
                        }
                    } else {
                        warn!("Invalid bootstrap node address format: {}", addr);
                        None
                    }
                }
            })
            .collect()
    }

    /// Register a .anon service and publish its descriptor to the DHT
    ///
    /// This generates a keypair for the service, creates introduction points,
    /// signs the descriptor, and publishes it to the network.
    ///
    /// # Arguments
    /// * `local_host` - Local host/IP where the service is running (e.g., "127.0.0.1")
    /// * `local_port` - Local port where the service is running (e.g., 8080)
    /// * `ttl_hours` - How long the descriptor should be valid (1-24 hours)
    ///
    /// # Returns
    /// Tuple of (service_address, keypair) - Save the keypair to reuse for updates
    pub async fn register_service(
        &self,
        local_host: String,
        local_port: u16,
        ttl_hours: u64,
    ) -> Result<(crate::service::ServiceAddress, KeyPair)> {
        use crate::service::descriptor::{ConnectionInfo, IntroductionPoint};
        use crate::service::ServiceDescriptor;
        use std::time::Duration;

        info!("Registering .anon service for {}:{}", local_host, local_port);

        // 1. Generate keypair for the service
        let service_keypair = KeyPair::generate();
        let service_address = crate::service::ServiceAddress::from_public_key(&service_keypair.public_key());

        info!("Generated .anon address: {}", service_address.to_hostname());

        // 2. Select introduction points from connected peers
        let introduction_points = self.select_introduction_points(&service_address).await?;

        if introduction_points.is_empty() {
            return Err(anyhow::anyhow!(
                "No connected peers available for introduction points. Connect to the network first."
            ));
        }

        info!("Selected {} introduction points", introduction_points.len());

        // 3. Create service descriptor
        let ttl = Duration::from_secs(ttl_hours * 3600);
        let mut descriptor = ServiceDescriptor::new(
            service_keypair.public_key(),
            introduction_points,
            ttl,
        );

        // 4. Sign the descriptor
        descriptor.sign(&service_keypair);

        info!("Signed service descriptor");

        // 5. Publish to DHT
        self.service_directory
            .publish_descriptor(descriptor)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish descriptor: {}", e))?;

        info!("Published service descriptor to DHT");
        info!("Service is now accessible at: {}", service_address.to_hostname());

        Ok((service_address, service_keypair))
    }

    /// Select introduction points from connected peers
    ///
    /// Chooses 3-5 reliable connected peers to act as introduction points
    async fn select_introduction_points(
        &self,
        _service_address: &crate::service::ServiceAddress,
    ) -> Result<Vec<crate::service::descriptor::IntroductionPoint>> {
        use crate::service::descriptor::{ConnectionInfo, IntroductionPoint};

        let peer_manager = self.peer_manager.read().await;
        let peers = peer_manager.connected_peers();

        if peers.is_empty() {
            return Ok(Vec::new());
        }

        // Select up to 3 introduction points (Tor uses 3)
        let count = std::cmp::min(3, peers.len());
        let selected_peers = &peers[0..count];

        let mut intro_points = Vec::new();

        for peer in selected_peers {
            // Get the first address (or fallback to localhost)
            let (ip_addr, port) = if let Some(first_addr) = peer.addresses.first() {
                match first_addr {
                    anonnet_common::NetworkAddress::Socket(socket_addr) => {
                        (socket_addr.ip().to_string(), socket_addr.port())
                    }
                    anonnet_common::NetworkAddress::Domain { host, port } => {
                        (host.clone(), *port)
                    }
                }
            } else {
                ("127.0.0.1".to_string(), 9000)
            };

            let connection_info = ConnectionInfo {
                addresses: vec![ip_addr],
                port,
                protocol_version: 1,
            };

            // NOTE: In a full implementation, we would:
            // 1. Send a request to the peer asking permission to use them as intro point
            // 2. Peer would respond with their signature
            // 3. We'd include that signature in the IntroductionPoint
            //
            // For testing, we'll create unsigned intro points (validation will be relaxed)
            let intro_point = IntroductionPoint::new(
                peer.node_id,
                peer.public_key,
                connection_info,
            );

            intro_points.push(intro_point);
        }

        Ok(intro_points)
    }

    /// Look up a service descriptor by .anon address
    pub async fn lookup_service(
        &self,
        address: &crate::service::ServiceAddress,
    ) -> Result<crate::service::ServiceDescriptor> {
        self.service_directory
            .lookup_descriptor(address)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to lookup service: {}", e))
    }

    /// Get all locally published services
    pub async fn get_published_services(&self) -> Vec<crate::service::ServiceDescriptor> {
        self.service_directory.get_cached_descriptors().await
    }
}

/// Node statistics
#[derive(Debug, Clone)]
pub struct NodeStats {
    pub node_id: NodeId,
    pub peer_count: usize,
    pub active_peers: usize,
    pub circuits: usize,
    pub active_circuits: usize,
    pub bandwidth: u64,
    pub is_running: bool,
}

/// Credit statistics
#[derive(Debug, Clone)]
pub struct CreditStats {
    /// Total credits earned from relaying
    pub total_earned: u64,
    /// Total credits spent on circuits
    pub total_spent: u64,
    /// Current earning rate (credits per hour)
    pub earning_rate: f64,
    /// Current spending rate (credits per hour)
    pub spending_rate: f64,
}

/// Active circuit information
#[derive(Debug, Clone)]
pub struct ActiveCircuitInfo {
    /// Circuit ID
    pub circuit_id: crate::circuit::CircuitId,
    /// Circuit purpose
    pub purpose: crate::circuit::CircuitPurpose,
    /// Circuit state
    pub state: crate::circuit::CircuitState,
    /// Number of hops
    pub hops: usize,
    /// Age in seconds
    pub age_seconds: u64,
    /// Number of times used
    pub use_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_creation() {
        let config = NodeConfig::default();
        let result = Node::new(config).await;
        assert!(result.is_ok());
    }
}
