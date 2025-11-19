/// SOCKS5 proxy server for routing traffic through AnonNet
///
/// This module implements a SOCKS5 proxy server that allows applications
/// to route their traffic through the anonymous network.

use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use tracing::{debug, error, info};

/// SOCKS5 proxy server
pub struct Socks5Server {
    listen_addr: SocketAddr,
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
    /// Create a new SOCKS5 server
    pub fn new(listen_addr: SocketAddr) -> Self {
        Self { listen_addr }
    }

    /// Start the SOCKS5 proxy server
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(self.listen_addr).await?;
        info!("SOCKS5 proxy listening on {}", self.listen_addr);

        loop {
            let (socket, addr) = listener.accept().await?;
            debug!("SOCKS5: New connection from {}", addr);

            tokio::spawn(async move {
                if let Err(e) = handle_client(socket).await {
                    error!("SOCKS5 error: {}", e);
                }
            });
        }
    }
}

/// Handle a SOCKS5 client connection
async fn handle_client(mut stream: TcpStream) -> Result<()> {
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

    debug!("SOCKS5: Connecting to {}", target);

    // TODO: Route through AnonNet circuit instead of direct connection
    // For now, make direct connection as a placeholder
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_socks5_server_creation() {
        let addr: SocketAddr = "127.0.0.1:1080".parse().unwrap();
        let server = Socks5Server::new(addr);
        assert_eq!(server.listen_addr, addr);
    }
}
