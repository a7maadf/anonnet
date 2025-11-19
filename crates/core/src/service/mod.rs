/// .anon service system
///
/// This module provides the infrastructure for anonymous services:
/// - Service addresses (like Tor's .onion)
/// - Service descriptors (introduction points, metadata)
/// - Service directory (DHT-based discovery)
/// - Rendezvous system (anonymous connections)

pub mod address;
pub mod descriptor;
pub mod directory;
pub mod rendezvous;

pub use address::{ServiceAddress, ServiceAddressError};
pub use descriptor::{
    ConnectionInfo, DescriptorError, IntroductionPoint, ServiceDescriptor,
};
pub use directory::{DirectoryError, ServiceDirectory};
pub use rendezvous::{RendezvousError, RendezvousId, RendezvousManager};
