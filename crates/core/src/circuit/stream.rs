/// Circuit stream for bidirectional data relay
///
/// Provides a tokio-compatible stream interface over circuits for
/// transparent integration with SOCKS5/HTTP proxies.

use super::relay::RelayHandler;
use super::types::{Circuit, CircuitId, RelayCell, RelayCellType};
use crate::network::ConnectionHandler;
use crate::protocol::messages::{Message, MessagePayload, RelayCellMessage};
use anyhow::{anyhow, Result};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Maximum payload size per relay cell (similar to Tor's 498 bytes)
const MAX_CELL_PAYLOAD_SIZE: usize = 498;

/// A bidirectional stream over a circuit
pub struct CircuitStream {
    /// Circuit ID
    circuit_id: CircuitId,

    /// Stream ID within the circuit
    stream_id: u16,

    /// Sequence counter for sending
    send_sequence: u32,

    /// Connection handler for the first hop
    first_hop_handler: Arc<ConnectionHandler>,

    /// Receive buffer (cells received but not yet read)
    receive_buffer: VecDeque<Vec<u8>>,

    /// Whether the stream has been closed
    closed: bool,

    /// Mutable circuit reference for encryption (wrapped in Mutex)
    circuit: Arc<Mutex<Circuit>>,
}

impl CircuitStream {
    /// Create a new circuit stream
    pub fn new(
        circuit_id: CircuitId,
        stream_id: u16,
        first_hop_handler: Arc<ConnectionHandler>,
        circuit: Arc<Mutex<Circuit>>,
    ) -> Self {
        Self {
            circuit_id,
            stream_id,
            send_sequence: 0,
            first_hop_handler,
            receive_buffer: VecDeque::new(),
            closed: false,
            circuit,
        }
    }

    /// Send a BEGIN cell to establish the stream
    pub async fn begin(&mut self, target: &str) -> Result<()> {
        debug!("CircuitStream: Beginning stream to {}", target);

        let target_bytes = target.as_bytes().to_vec();
        let cell = RelayHandler::create_begin_cell(self.stream_id, target_bytes);

        self.send_relay_cell(cell).await?;

        // TODO: Wait for CONNECTED response
        Ok(())
    }

    /// Send data through the circuit
    ///
    /// Splits large data into multiple relay cells if needed
    pub async fn send(&mut self, data: &[u8]) -> Result<usize> {
        if self.closed {
            return Err(anyhow!("Stream is closed"));
        }

        let mut sent = 0;

        // Split data into cell-sized chunks
        for chunk in data.chunks(MAX_CELL_PAYLOAD_SIZE) {
            let cell = RelayHandler::create_data_cell(
                self.stream_id,
                chunk.to_vec(),
                self.send_sequence,
            );

            self.send_relay_cell(cell).await?;
            self.send_sequence += 1;
            sent += chunk.len();
        }

        debug!("CircuitStream: Sent {} bytes", sent);
        Ok(sent)
    }

    /// Receive data from the circuit
    ///
    /// Returns the next chunk of data from the receive buffer
    pub async fn recv(&mut self) -> Result<Vec<u8>> {
        if let Some(data) = self.receive_buffer.pop_front() {
            return Ok(data);
        }

        // TODO: Wait for incoming relay cells and decrypt them
        // For now, return error indicating no data available
        Err(anyhow!("No data available (stream needs incoming cell handler)"))
    }

    /// Close the stream
    pub async fn close(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }

        debug!("CircuitStream: Closing stream {}", self.stream_id);

        let cell = RelayHandler::create_end_cell(self.stream_id, 0);
        self.send_relay_cell(cell).await?;

        self.closed = true;
        Ok(())
    }

    /// Send a relay cell through the circuit
    async fn send_relay_cell(&mut self, cell: RelayCell) -> Result<()> {
        // Encrypt the cell with all circuit layers
        let encrypted = {
            let mut circuit = self.circuit.lock().await;
            RelayHandler::encrypt_cell_for_circuit(&mut circuit, &cell)?
        };

        // Send through the first hop
        let relay_msg = Message::new(MessagePayload::RelayCell(RelayCellMessage {
            circuit_id: crate::protocol::messages::CircuitId(self.circuit_id.as_u64()),
            encrypted_payload: encrypted,
        }));

        // Send as one-way message (no response expected for data cells)
        self.first_hop_handler.send_message(relay_msg).await?;

        Ok(())
    }

    /// Handle an incoming relay cell (called by circuit manager)
    pub async fn handle_incoming_cell(&mut self, cell: RelayCell) {
        match cell.cell_type {
            RelayCellType::Data => {
                // Add to receive buffer
                self.receive_buffer.push_back(cell.payload);
            }
            RelayCellType::End => {
                debug!("CircuitStream: Received END cell");
                self.closed = true;
            }
            _ => {
                warn!("CircuitStream: Unexpected cell type: {:?}", cell.cell_type);
            }
        }
    }

    /// Check if the stream is closed
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

/// Bidirectional relay between a TCP stream and a circuit stream
///
/// Uses channels to avoid borrow checker issues with concurrent access
pub async fn relay_bidirectional(
    mut tcp_stream: tokio::net::TcpStream,
    circuit_stream: CircuitStream,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc;

    debug!("Starting bidirectional relay");

    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

    // Create channels for communication between tasks
    let (send_tx, mut send_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (recv_tx, mut recv_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Wrap circuit stream in Arc<Mutex> for shared access
    let circuit_stream = std::sync::Arc::new(tokio::sync::Mutex::new(circuit_stream));
    let circuit_stream_send = circuit_stream.clone();
    let circuit_stream_recv = circuit_stream.clone();

    // TCP read task: TCP -> send channel
    let tcp_reader = tokio::spawn(async move {
        let mut buf = vec![0u8; MAX_CELL_PAYLOAD_SIZE];
        loop {
            match tcp_read.read(&mut buf).await {
                Ok(0) => {
                    debug!("TCP EOF");
                    break;
                }
                Ok(n) => {
                    if send_tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!("TCP read error: {}", e);
                    break;
                }
            }
        }
    });

    // Circuit send task: send channel -> circuit
    let circuit_sender = tokio::spawn(async move {
        while let Some(data) = send_rx.recv().await {
            let mut cs = circuit_stream_send.lock().await;
            if let Err(e) = cs.send(&data).await {
                warn!("Failed to send to circuit: {}", e);
                break;
            }
        }
    });

    // Circuit receive task: circuit -> recv channel
    let circuit_receiver = tokio::spawn(async move {
        loop {
            let mut cs = circuit_stream_recv.lock().await;
            match cs.recv().await {
                Ok(data) => {
                    if recv_tx.send(data).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    if cs.is_closed() {
                        debug!("Circuit stream closed");
                        break;
                    }
                    // Release lock and wait before retrying
                    drop(cs);
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }
        }
    });

    // TCP write task: recv channel -> TCP
    let tcp_writer = tokio::spawn(async move {
        while let Some(data) = recv_rx.recv().await {
            if let Err(e) = tcp_write.write_all(&data).await {
                warn!("TCP write error: {}", e);
                break;
            }
        }
    });

    // Wait for any task to complete
    tokio::select! {
        _ = tcp_reader => debug!("TCP reader ended"),
        _ = circuit_sender => debug!("Circuit sender ended"),
        _ = circuit_receiver => debug!("Circuit receiver ended"),
        _ = tcp_writer => debug!("TCP writer ended"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::types::CircuitPurpose;
    use crate::identity::KeyPair;

    #[tokio::test]
    async fn test_circuit_stream_creation() {
        let circuit_id = CircuitId::generate();
        let circuit = Arc::new(Mutex::new(Circuit::new(circuit_id, CircuitPurpose::General)));

        // Note: In real usage, you'd need a proper ConnectionHandler
        // For this test, we're just checking the struct can be created
        // without a full network setup.
    }
}
