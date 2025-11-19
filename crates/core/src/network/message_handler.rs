/// Network message handler for sending and receiving protocol messages
///
/// This module provides the core message handling functionality:
/// - Serialization/deserialization of messages over QUIC streams
/// - Message dispatching to appropriate handlers
/// - Peer authentication and connection management

use crate::protocol::messages::{ErrorCode, ErrorMessage, Message, MessagePayload, PingMessage};
use crate::transport::{Connection, RecvStream, SendStream};
use anyhow::Result;
use tracing::{debug, error, warn};

/// Maximum message size (10 MB)
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Message codec for sending and receiving messages over streams
pub struct MessageCodec;

impl MessageCodec {
    /// Send a message over a stream
    pub async fn send_message(send: &mut SendStream, message: &Message) -> Result<()> {
        // Serialize message to JSON (more readable for debugging, can switch to bincode for production)
        let serialized = serde_json::to_vec(message)?;

        // Send length prefix (4 bytes, little-endian)
        let len = serialized.len() as u32;
        send.write(&len.to_le_bytes()).await?;

        // Send message data
        send.write(&serialized).await?;

        debug!("Sent {} message ({} bytes)", message.message_type(), serialized.len());
        Ok(())
    }

    /// Receive a message from a stream
    pub async fn recv_message(recv: &mut RecvStream) -> Result<Option<Message>> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        match recv.read_exact(&mut len_buf).await {
            Ok(()) => {},
            Err(e) => {
                // Stream closed or error
                debug!("Stream closed or error reading length: {}", e);
                return Ok(None);
            }
        }

        let len = u32::from_le_bytes(len_buf) as usize;

        // Validate message size
        if len > MAX_MESSAGE_SIZE {
            error!("Message too large: {} bytes (max {})", len, MAX_MESSAGE_SIZE);
            return Err(anyhow::anyhow!("Message too large"));
        }

        // Read message data
        let data = recv.read_to_end(len).await?;

        if data.len() != len {
            error!("Incomplete message: expected {} bytes, got {}", len, data.len());
            return Err(anyhow::anyhow!("Incomplete message"));
        }

        // Deserialize message
        let message: Message = serde_json::from_slice(&data)?;

        debug!("Received {} message ({} bytes)", message.message_type(), len);
        Ok(Some(message))
    }

    /// Send a message and close the stream
    pub async fn send_message_and_finish(mut send: SendStream, message: &Message) -> Result<()> {
        Self::send_message(&mut send, message).await?;
        send.finish().await?;
        Ok(())
    }

    /// Send a message over a new bidirectional stream and wait for response
    pub async fn send_request(
        connection: &Connection,
        request: Message,
    ) -> Result<Message> {
        let (mut send, mut recv) = connection.open_bi().await?;

        Self::send_message(&mut send, &request).await?;
        send.finish().await?;

        let response = Self::recv_message(&mut recv).await?
            .ok_or_else(|| anyhow::anyhow!("No response received"))?;

        Ok(response)
    }
}

/// Connection handler for a single peer
#[derive(Debug)]
pub struct ConnectionHandler {
    connection: Connection,
}

impl ConnectionHandler {
    /// Create a new connection handler
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    /// Send a message to the peer
    pub async fn send_message(&self, message: Message) -> Result<()> {
        let (mut send, _recv) = self.connection.open_bi().await?;
        MessageCodec::send_message_and_finish(send, &message).await
    }

    /// Send a request and wait for response
    pub async fn send_request(&self, request: Message) -> Result<Message> {
        MessageCodec::send_request(&self.connection, request).await
    }

    /// Accept and handle incoming messages in a loop
    pub async fn handle_incoming_messages<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(Message) -> Result<Option<Message>>,
    {
        loop {
            // Accept incoming stream
            let (mut send, mut recv) = match self.connection.accept_bi().await {
                Ok(streams) => streams,
                Err(e) => {
                    debug!("Connection closed: {}", e);
                    break;
                }
            };

            // Receive message
            let message = match MessageCodec::recv_message(&mut recv).await {
                Ok(Some(msg)) => msg,
                Ok(None) => {
                    debug!("Stream closed without message");
                    continue;
                }
                Err(e) => {
                    warn!("Failed to receive message: {}", e);
                    continue;
                }
            };

            // Handle message
            match handler(message) {
                Ok(Some(response)) => {
                    // Send response
                    if let Err(e) = MessageCodec::send_message_and_finish(send, &response).await {
                        warn!("Failed to send response: {}", e);
                    }
                }
                Ok(None) => {
                    // No response needed
                    let _ = send.finish().await;
                }
                Err(e) => {
                    error!("Error handling message: {}", e);

                    // Send error response
                    let error_msg = Message::new(MessagePayload::Error(ErrorMessage {
                        code: ErrorCode::InternalError,
                        message: e.to_string(),
                    }));

                    let _ = MessageCodec::send_message_and_finish(send, &error_msg).await;
                }
            }
        }

        Ok(())
    }

    /// Get the underlying connection
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{Endpoint, EndpointConfig};

    #[tokio::test]
    async fn test_send_receive_message() {
        // Create server and client endpoints
        let server_config = EndpointConfig::default();
        let server = Endpoint::new(server_config).await.unwrap();
        let server_addr = server.local_addr();

        let client_config = EndpointConfig::default();
        let client = Endpoint::new(client_config).await.unwrap();

        // Spawn server task
        let server_task = tokio::spawn(async move {
            let conn = server.accept().await.unwrap();
            let (_send, mut recv) = conn.accept_bi().await.unwrap();
            MessageCodec::recv_message(&mut recv).await.unwrap()
        });

        // Client connects and sends message
        let client_conn = client.connect(server_addr).await.unwrap();
        let (mut send, _recv) = client_conn.open_bi().await.unwrap();

        let message = Message::new(MessagePayload::Ping(PingMessage { nonce: 12345 }));
        MessageCodec::send_message(&mut send, &message).await.unwrap();
        send.finish().await.unwrap();

        // Server receives message
        let received = server_task.await.unwrap().unwrap();

        assert_eq!(message.message_id, received.message_id);
        match received.payload {
            MessagePayload::Ping(ping) => assert_eq!(ping.nonce, 12345),
            _ => panic!("Wrong message type"),
        }
    }
}
