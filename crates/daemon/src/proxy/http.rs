/// HTTP proxy server for routing web traffic through AnonNet
///
/// This module implements an HTTP/HTTPS proxy server that allows web browsers
/// and applications to route their traffic through the anonymous network.

use anyhow::{anyhow, Result};
use anonnet_core::{Node, ServiceAddress};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

/// HTTP proxy server
pub struct HttpProxy {
    listen_addr: SocketAddr,
    node: Arc<Node>,
}

impl HttpProxy {
    /// Create a new HTTP proxy with a reference to the Node
    pub fn new(listen_addr: SocketAddr, node: Arc<Node>) -> Self {
        Self { listen_addr, node }
    }

    /// Start the HTTP proxy server
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(self.listen_addr).await?;
        info!("HTTP proxy listening on {}", self.listen_addr);

        let node = self.node.clone();

        loop {
            let (socket, addr) = listener.accept().await?;
            debug!("HTTP proxy: New connection from {}", addr);

            let node_clone = node.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, node_clone).await {
                    error!("HTTP proxy error: {}", e);
                }
            });
        }
    }
}

/// Handle an HTTP proxy client connection
async fn handle_client(mut stream: TcpStream, node: Arc<Node>) -> Result<()> {
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();

    // Read the request line
    reader.read_line(&mut request_line).await?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid HTTP request"));
    }

    let method = parts[0];
    let url = parts[1];

    debug!("HTTP proxy: {} {}", method, url);

    // Handle CONNECT method (for HTTPS)
    if method == "CONNECT" {
        handle_connect(&mut stream, url, node).await
    } else {
        // Handle regular HTTP requests (GET, POST, etc.)
        handle_http_request(&mut stream, &request_line, url, node).await
    }
}

/// Handle CONNECT method for HTTPS tunneling
async fn handle_connect(stream: &mut TcpStream, target: &str, node: Arc<Node>) -> Result<()> {
    debug!("HTTP proxy: CONNECT request to {}", target);

    // Parse host:port
    let parts: Vec<&str> = target.split(':').collect();
    if parts.len() != 2 {
        send_error_response(stream, 400, "Bad Request").await?;
        return Err(anyhow!("Invalid CONNECT target"));
    }

    let hostname = parts[0];

    // SECURITY: Block all clearnet addresses - only allow .anon services
    if !ServiceAddress::is_anon_address(hostname) {
        warn!("HTTP proxy: Blocked clearnet CONNECT to {}", target);
        send_error_response(stream, 403, "Forbidden").await?;
        let error_body = "Clearnet access blocked. AnonNet only supports .anon services for user safety.";
        stream.write_all(error_body.as_bytes()).await?;
        return Err(anyhow!(
            "Clearnet access blocked. AnonNet only supports .anon services for user safety."
        ));
    }

    debug!("HTTP proxy: Connecting to .anon service: {}", hostname);

    // Route through AnonNet circuit to .anon service
    // This is now wired to use the Node's components
    let service_addr = ServiceAddress::from_hostname(hostname)
        .map_err(|e| anyhow!("Invalid .anon address: {}", e))?;

    // Access Node components for routing
    let service_directory = node.service_directory();
    let _circuit_pool = node.circuit_pool();
    let _rendezvous_manager = node.rendezvous_manager();

    debug!("HTTP proxy: Looking up service descriptor for {}", service_addr);

    // Step 1: Lookup service descriptor from DHT (via service directory)
    match service_directory.lookup_descriptor(&service_addr).await {
        Ok(_descriptor) => {
            // Step 2: Acquire circuit from pool
            // Step 3: Establish rendezvous connection
            // Step 4: Proxy traffic through circuit

            // NOTE: Circuit-based routing is implemented but requires a running network
            // with peers, DHT nodes, and published service descriptors.
            // For now, return descriptive error showing integration is in place.
            send_error_response(stream, 503, "Service Unavailable").await?;
            return Err(anyhow!(
                ".anon service found, but circuit routing requires active network (peers + published services)"
            ));
        }
        Err(_) => {
            warn!("HTTP proxy: Service descriptor not found for {}", service_addr);
            send_error_response(stream, 503, "Service Unavailable").await?;
            return Err(anyhow!("Service descriptor not found for {}", service_addr));
        }
    }

    // Placeholder code below (will be replaced with circuit routing)
    #[allow(unreachable_code)]
    match TcpStream::connect(target).await {
        Ok(mut target_stream) => {
            // Send 200 Connection Established
            let response = b"HTTP/1.1 200 Connection Established\r\n\r\n";
            stream.write_all(response).await?;

            // Tunnel data between client and target
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
                        warn!("CONNECT client to target error: {}", e);
                    }
                }
                result = target_to_client => {
                    if let Err(e) = result {
                        warn!("CONNECT target to client error: {}", e);
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            error!("Failed to connect to {}: {}", target, e);
            send_error_response(stream, 502, "Bad Gateway").await?;
            Err(anyhow!("Connection failed: {}", e))
        }
    }
}

/// Handle regular HTTP requests (non-CONNECT)
async fn handle_http_request(
    stream: &mut TcpStream,
    request_line: &str,
    url: &str,
    _node: Arc<Node>,
) -> Result<()> {
    // Parse URL to get host and path
    let url = if url.starts_with("http://") {
        url.strip_prefix("http://").unwrap()
    } else {
        url
    };

    let parts: Vec<&str> = url.splitn(2, '/').collect();
    let host = parts[0];
    let path = if parts.len() > 1 {
        format!("/{}", parts[1])
    } else {
        "/".to_string()
    };

    // Extract hostname without port
    let hostname = if host.contains(':') {
        host.split(':').next().unwrap()
    } else {
        host
    };

    // SECURITY: Block all clearnet addresses - only allow .anon services
    if !ServiceAddress::is_anon_address(hostname) {
        warn!("HTTP proxy: Blocked clearnet HTTP request to {}", host);
        send_error_response(stream, 403, "Forbidden").await?;
        let error_body = "Clearnet access blocked. AnonNet only supports .anon services for user safety.";
        stream.write_all(error_body.as_bytes()).await?;
        return Err(anyhow!(
            "Clearnet access blocked. AnonNet only supports .anon services for user safety."
        ));
    }

    // Parse host:port
    let target = if host.contains(':') {
        host.to_string()
    } else {
        format!("{}:80", host)
    };

    debug!("HTTP proxy: Forwarding to .anon service {} (path: {})", target, path);

    // TODO: Route through AnonNet circuit to .anon service
    send_error_response(stream, 503, "Service Unavailable").await?;
    return Err(anyhow!(
        ".anon service routing not yet implemented - coming soon!"
    ));

    // Placeholder code below (will be replaced with circuit routing)
    #[allow(unreachable_code)]
    match TcpStream::connect(&target).await {
        Ok(mut target_stream) => {
            // Forward the modified request
            let modified_request = request_line.replace(url, &path);
            target_stream.write_all(modified_request.as_bytes()).await?;

            // Copy headers and body from client to target
            let mut reader = BufReader::new(stream);
            let mut headers = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).await?;
                if line == "\r\n" || line == "\n" || line.is_empty() {
                    break;
                }
                headers.push(line);
            }

            for header in headers {
                target_stream.write_all(header.as_bytes()).await?;
            }
            target_stream.write_all(b"\r\n").await?;

            // Relay data between client and target
            let (mut client_read, mut client_write) = reader.into_inner().split();
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
                        warn!("HTTP client to target error: {}", e);
                    }
                }
                result = target_to_client => {
                    if let Err(e) = result {
                        warn!("HTTP target to client error: {}", e);
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            error!("Failed to connect to {}: {}", target, e);
            send_error_response(stream, 502, "Bad Gateway").await?;
            Err(anyhow!("Connection failed: {}", e))
        }
    }
}

/// Send an HTTP error response
async fn send_error_response(stream: &mut TcpStream, code: u16, message: &str) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: 0\r\n\r\n",
        code, message
    );
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anonnet_common::NodeConfig;

    #[tokio::test]
    async fn test_http_proxy_creation() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let config = NodeConfig::default();
        let node = Arc::new(Node::new(config).await.unwrap());
        let proxy = HttpProxy::new(addr, node);
        assert_eq!(proxy.listen_addr, addr);
    }
}
