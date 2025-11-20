/// SOCKS5 proxy server for routing traffic through AnonNet
///
/// This module implements a SOCKS5 proxy server that allows applications
/// to route their traffic through the anonymous network.

use anyhow::{anyhow, Result};
use anonnet_core::{Node, ServiceAddress};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

/// SOCKS5 proxy server
pub struct Socks5Server {
    listen_addr: SocketAddr,
    node: Arc<Node>,
}

/// SOCKS5 protocol constants
const SOCKS_VERSION: u8 = 0x05;
const NO_AUTH_REQUIRED: u8 = 0x00;
const CONNECT_COMMAND: u8 = 0x01;
const IPV4_ADDRESS: u8 = 0x01;
const DOMAIN_NAME: u8 = 0x03;
const IPV6_ADDRESS: u8 = 0x04;

/// Reply codes
const SUCCESS: u8 = 0x00;
const GENERAL_FAILURE: u8 = 0x01;
const CONNECTION_NOT_ALLOWED: u8 = 0x02;
const NETWORK_UNREACHABLE: u8 = 0x03;
const HOST_UNREACHABLE: u8 = 0x04;
const CONNECTION_REFUSED: u8 = 0x05;
const COMMAND_NOT_SUPPORTED: u8 = 0x07;
const ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;

impl Socks5Server {
    /// Create a new SOCKS5 server with a reference to the Node
    pub fn new(listen_addr: SocketAddr, node: Arc<Node>) -> Self {
        Self { listen_addr, node }
    }

    /// Start the SOCKS5 proxy server
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(self.listen_addr).await?;
        let actual_addr = listener.local_addr()?;

        info!("╔═══════════════════════════════════════════════════╗");
        info!("║   SOCKS5 Proxy Started on {}   ║", actual_addr);
        info!("╚═══════════════════════════════════════════════════╝");

        // Save port to file for browser to discover
        if let Err(e) = std::fs::write("./data/socks5_port.txt", actual_addr.port().to_string()) {
            warn!("Failed to write SOCKS5 port file: {}", e);
        }

        let node = self.node.clone();

        loop {
            let (socket, addr) = listener.accept().await?;
            debug!("SOCKS5: New connection from {}", addr);

            let node_clone = node.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, node_clone).await {
                    error!("SOCKS5 error: {}", e);
                }
            });
        }
    }
}

/// Handle a SOCKS5 client connection
async fn handle_client(mut stream: TcpStream, node: Arc<Node>) -> Result<()> {
    // 1. Handshake
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;

    if buf[0] != SOCKS_VERSION {
        return Err(anyhow!("Unsupported SOCKS version: {}", buf[0]));
    }

    let n_methods = buf[1] as usize;
    let mut methods = vec![0u8; n_methods];
    stream.read_exact(&mut methods).await?;

    // We only support NO_AUTH for now
    if !methods.contains(&NO_AUTH_REQUIRED) {
        stream.write_all(&[SOCKS_VERSION, 0xFF]).await?;
        return Err(anyhow!("No acceptable auth methods"));
    }

    // Send auth method selection
    stream.write_all(&[SOCKS_VERSION, NO_AUTH_REQUIRED]).await?;

    // 2. Request
    let mut request = [0u8; 4];
    stream.read_exact(&mut request).await?;

    if request[0] != SOCKS_VERSION {
        return Err(anyhow!("Invalid SOCKS version in request"));
    }

    let command = request[1];
    let address_type = request[3];

    // We only support CONNECT for now
    if command != CONNECT_COMMAND {
        send_reply(&mut stream, COMMAND_NOT_SUPPORTED).await?;
        return Err(anyhow!("Unsupported command: {}", command));
    }

    // Parse target address
    let target = match address_type {
        IPV4_ADDRESS => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await?;
            let mut port = [0u8; 2];
            stream.read_exact(&mut port).await?;
            let port = u16::from_be_bytes(port);

            format!("{}.{}.{}.{}:{}", addr[0], addr[1], addr[2], addr[3], port)
        }
        DOMAIN_NAME => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize];
            stream.read_exact(&mut domain).await?;
            let mut port = [0u8; 2];
            stream.read_exact(&mut port).await?;
            let port = u16::from_be_bytes(port);

            format!("{}:{}", String::from_utf8_lossy(&domain), port)
        }
        IPV6_ADDRESS => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await?;
            let mut port = [0u8; 2];
            stream.read_exact(&mut port).await?;
            let port = u16::from_be_bytes(port);

            // Format IPv6 address
            format!("[{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}]:{}",
                addr[0], addr[1], addr[2], addr[3], addr[4], addr[5], addr[6], addr[7],
                addr[8], addr[9], addr[10], addr[11], addr[12], addr[13], addr[14], addr[15], port)
        }
        _ => {
            send_reply(&mut stream, ADDRESS_TYPE_NOT_SUPPORTED).await?;
            return Err(anyhow!("Unsupported address type: {}", address_type));
        }
    };

    debug!("SOCKS5: Request to connect to {}", target);

    // Extract hostname from target
    let hostname = extract_hostname(&target);

    // SECURITY: Block all clearnet addresses - only allow .anon services
    if !ServiceAddress::is_anon_address(&hostname) {
        warn!("SOCKS5: Blocked clearnet address: {}", target);
        send_reply(&mut stream, CONNECTION_NOT_ALLOWED).await?;
        return Err(anyhow!(
            "Clearnet access blocked. AnonNet only supports .anon services for user safety."
        ));
    }

    debug!("SOCKS5: Connecting to .anon service: {}", hostname);

    // Route through AnonNet circuit to .anon service
    let service_addr = ServiceAddress::from_hostname(&hostname)
        .map_err(|e| anyhow!("Invalid .anon address: {}", e))?;

    // Access Node components for routing
    let service_directory = node.service_directory();
    let circuit_pool = node.circuit_pool();
    let routing_table = node.routing_table();
    let circuit_manager = node.circuit_manager();
    let connection_manager = node.connection_manager()
        .ok_or_else(|| anyhow!("Connection manager not initialized"))?;

    debug!("SOCKS5: Looking up service descriptor for {}", service_addr);

    // Step 1: Lookup service descriptor from DHT (via service directory)
    let _descriptor = service_directory.lookup_descriptor(&service_addr).await
        .map_err(|e| {
            warn!("SOCKS5: Service descriptor not found for {}: {}", service_addr, e);
            e
        })?;

    debug!("SOCKS5: Service descriptor found");

    // Step 2: Acquire circuit from pool
    let circuit_id = {
        use anonnet_core::circuit::CircuitPurpose;
        let rt = routing_table.read().await;

        circuit_pool.acquire_circuit(CircuitPurpose::General, &*rt).await
            .map_err(|e| {
                warn!("SOCKS5: Failed to acquire circuit: {}", e);
                anyhow!("Failed to acquire circuit: {}. Ensure DHT has discovered peers first.", e)
            })?
    };

    debug!("SOCKS5: Acquired circuit {}", circuit_id);

    // Step 3: Get the circuit and first hop handler
    let (circuit_arc, first_hop_id) = {
        let manager = circuit_manager.read().await;
        let circuit = manager.get_circuit(&circuit_id)
            .ok_or_else(|| anyhow!("Circuit not found"))?;

        let first_hop_id = circuit.entry_node()
            .ok_or_else(|| anyhow!("Circuit has no entry node"))?
            .node_id;

        // Clone the circuit into an Arc<Mutex> for the stream
        use tokio::sync::Mutex;
        let circuit_arc = std::sync::Arc::new(Mutex::new(circuit.clone()));

        (circuit_arc, first_hop_id)
    };

    // Get connection handler for first hop
    let first_hop_handler = connection_manager.get_connection(&first_hop_id)
        .ok_or_else(|| anyhow!("No connection to first hop"))?;

    debug!("SOCKS5: Creating circuit stream");

    // Step 4: Create circuit stream
    use anonnet_core::circuit::CircuitStream;
    let stream_id = 1; // TODO: Proper stream ID allocation
    let mut circuit_stream = CircuitStream::new(
        circuit_id,
        stream_id,
        first_hop_handler,
        circuit_arc,
    );

    // Step 5: Begin the stream to the target
    circuit_stream.begin(&target).await
        .map_err(|e| {
            warn!("SOCKS5: Failed to begin circuit stream: {}", e);
            e
        })?;

    debug!("SOCKS5: Circuit stream established");

    // Send SUCCESS reply to SOCKS5 client
    send_reply(&mut stream, SUCCESS).await?;

    // Step 6: Relay traffic bidirectionally
    use anonnet_core::circuit::relay_bidirectional;
    relay_bidirectional(stream, circuit_stream).await
        .map_err(|e| {
            warn!("SOCKS5: Relay error: {}", e);
            e
        })?;

    info!("SOCKS5: Connection closed for {}", target);
    return Ok(());

    // Placeholder code below (will be replaced with circuit routing)
    #[allow(unreachable_code)]
    match TcpStream::connect(&target).await {
        Ok(mut target_stream) => {
            send_reply(&mut stream, SUCCESS).await?;

            // Relay data between client and target
            let (mut client_read, mut client_write) = stream.split();
            let (mut target_read, mut target_write) = target_stream.split();

            let client_to_target = async {
                tokio::io::copy(&mut client_read, &mut target_write).await
            };

            let target_to_client = async {
                tokio::io::copy(&mut target_read, &mut client_write).await
            };

            tokio::select! {
                result = client_to_target => {
                    if let Err(e) = result {
                        error!("Client to target relay error: {}", e);
                    }
                }
                result = target_to_client => {
                    if let Err(e) = result {
                        error!("Target to client relay error: {}", e);
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            error!("Failed to connect to target {}: {}", target, e);
            send_reply(&mut stream, HOST_UNREACHABLE).await?;
            Err(anyhow!("Connection failed: {}", e))
        }
    }
}

/// Send a SOCKS5 reply to the client
async fn send_reply(stream: &mut TcpStream, reply_code: u8) -> Result<()> {
    // Reply format: VER | REP | RSV | ATYP | BND.ADDR | BND.PORT
    let reply = [
        SOCKS_VERSION,
        reply_code,
        0x00, // Reserved
        IPV4_ADDRESS,
        0, 0, 0, 0, // Bind address (0.0.0.0)
        0, 0, // Bind port (0)
    ];

    stream.write_all(&reply).await?;
    Ok(())
}

/// Extract hostname from target address (removes port)
fn extract_hostname(target: &str) -> String {
    // Handle IPv6 addresses like [::1]:port
    if target.starts_with('[') {
        if let Some(end_bracket) = target.find(']') {
            return target[1..end_bracket].to_string();
        }
    }

    // Handle regular addresses like example.com:port or 1.2.3.4:port
    if let Some(colon_pos) = target.rfind(':') {
        return target[..colon_pos].to_string();
    }

    target.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anonnet_common::NodeConfig;

    #[tokio::test]
    async fn test_socks5_server_creation() {
        let addr: SocketAddr = "127.0.0.1:1080".parse().unwrap();
        let config = NodeConfig::default();
        let node = Arc::new(Node::new(config).await.unwrap());
        let server = Socks5Server::new(addr, node);
        assert_eq!(server.listen_addr, addr);
    }
}
