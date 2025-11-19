use quinn::Connection as QuinnConnection;
use std::net::SocketAddr;

/// A QUIC connection to a remote peer
#[derive(Debug)]
pub struct Connection {
    /// Quinn connection
    inner: QuinnConnection,
}

impl Connection {
    /// Create a new connection wrapper
    pub(crate) fn new(inner: QuinnConnection) -> Self {
        Self { inner }
    }

    /// Open a bidirectional stream
    pub async fn open_bi(&self) -> Result<(super::SendStream, super::RecvStream), ConnectionError> {
        let (send, recv) = self.inner
            .open_bi()
            .await
            .map_err(|e| ConnectionError::StreamOpen(e.to_string()))?;

        Ok((super::SendStream::new(send), super::RecvStream::new(recv)))
    }

    /// Open a unidirectional stream
    pub async fn open_uni(&self) -> Result<super::SendStream, ConnectionError> {
        let send = self.inner
            .open_uni()
            .await
            .map_err(|e| ConnectionError::StreamOpen(e.to_string()))?;

        Ok(super::SendStream::new(send))
    }

    /// Accept an incoming bidirectional stream
    pub async fn accept_bi(&self) -> Result<(super::SendStream, super::RecvStream), ConnectionError> {
        let (send, recv) = self.inner
            .accept_bi()
            .await
            .map_err(|e| ConnectionError::StreamAccept(e.to_string()))?;

        Ok((super::SendStream::new(send), super::RecvStream::new(recv)))
    }

    /// Accept an incoming unidirectional stream
    pub async fn accept_uni(&self) -> Result<super::RecvStream, ConnectionError> {
        let recv = self.inner
            .accept_uni()
            .await
            .map_err(|e| ConnectionError::StreamAccept(e.to_string()))?;

        Ok(super::RecvStream::new(recv))
    }

    /// Get remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.inner.remote_address()
    }

    /// Get connection statistics
    pub fn stats(&self) -> ConnectionStats {
        let quinn_stats = self.inner.stats();

        ConnectionStats {
            bytes_sent: quinn_stats.udp_tx.bytes,
            bytes_received: quinn_stats.udp_rx.bytes,
            datagrams_sent: quinn_stats.udp_tx.datagrams,
            datagrams_received: quinn_stats.udp_rx.datagrams,
        }
    }

    /// Close the connection gracefully
    pub fn close(&self, error_code: u32, reason: &str) {
        self.inner.close(error_code.into(), reason.as_bytes());
    }

    /// Check if connection is closed
    pub fn is_closed(&self) -> bool {
        self.inner.close_reason().is_some()
    }
}

/// Connection statistics
#[derive(Debug, Clone, Copy)]
pub struct ConnectionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub datagrams_sent: u64,
    pub datagrams_received: u64,
}

/// Connection errors
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Failed to open stream: {0}")]
    StreamOpen(String),

    #[error("Failed to accept stream: {0}")]
    StreamAccept(String),

    #[error("Connection closed: {0}")]
    Closed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{Endpoint, EndpointConfig};

    async fn create_connection_pair() -> (Connection, Connection) {
        // Create server
        let server_config = EndpointConfig::default();
        let server = Endpoint::new(server_config).await.unwrap();
        let server_addr = server.local_addr();

        // Create client
        let client_config = EndpointConfig::default();
        let client = Endpoint::new(client_config).await.unwrap();

        // Connect
        let accept_task = tokio::spawn(async move {
            server.accept().await
        });

        let client_conn = client.connect(server_addr).await.unwrap();
        let server_conn = accept_task.await.unwrap().unwrap();

        (client_conn, server_conn)
    }

    #[tokio::test]
    async fn test_connection_remote_addr() {
        let (client, server) = create_connection_pair().await;

        // Check remote addresses are correct
        assert_ne!(client.remote_addr().port(), 0);
        assert_ne!(server.remote_addr().port(), 0);
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let (client, _server) = create_connection_pair().await;

        let stats = client.stats();

        // Connection should have sent/received some data during handshake
        assert!(stats.bytes_sent > 0 || stats.bytes_received > 0);
    }

    #[tokio::test]
    async fn test_connection_close() {
        let (client, _server) = create_connection_pair().await;

        assert!(!client.is_closed());

        client.close(0, "test close");

        // Give it a moment to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert!(client.is_closed());
    }
}
