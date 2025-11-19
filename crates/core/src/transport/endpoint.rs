use quinn::Endpoint as QuinnEndpoint;
use std::net::SocketAddr;
use std::sync::Arc;

/// QUIC endpoint for creating connections
pub struct Endpoint {
    /// Quinn endpoint
    inner: QuinnEndpoint,

    /// Local address
    local_addr: SocketAddr,
}

impl Endpoint {
    /// Create a new endpoint
    pub async fn new(config: EndpointConfig) -> Result<Self, EndpointError> {
        let server_config = Self::create_server_config()?;
        let client_config = Self::create_client_config()?;

        let mut endpoint = QuinnEndpoint::server(server_config, config.bind_addr)
            .map_err(|e| EndpointError::BindFailed(e.to_string()))?;

        endpoint.set_default_client_config(client_config);

        let local_addr = endpoint.local_addr()?;

        Ok(Self {
            inner: endpoint,
            local_addr,
        })
    }

    /// Create server configuration with self-signed certificate
    fn create_server_config() -> Result<quinn::ServerConfig, EndpointError> {
        // Generate self-signed certificate
        let cert = rcgen::generate_simple_self_signed(vec!["anonnet.local".to_string()])
            .map_err(|e| EndpointError::CertGeneration(e.to_string()))?;

        let cert_der = cert.cert.der().to_vec();
        let key_der = cert.key_pair.serialize_der();

        let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der)];
        let key = rustls::pki_types::PrivateKeyDer::try_from(key_der)
            .map_err(|e| EndpointError::CertGeneration(format!("Invalid key: {:?}", e)))?;

        // Create server config with crypto provider
        let server_crypto = rustls::ServerConfig::builder_with_provider(
                Arc::new(rustls::crypto::ring::default_provider())
            )
            .with_safe_default_protocol_versions()
            .map_err(|e| EndpointError::ConfigCreation(format!("Failed to set protocol versions: {:?}", e)))?
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| EndpointError::ConfigCreation(e.to_string()))?;

        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
                .map_err(|e| EndpointError::ConfigCreation(format!("Failed to create QUIC server config: {:?}", e)))?
        ));

        // Configure transport
        let mut transport_config = quinn::TransportConfig::default();

        // Set timeouts
        transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into().unwrap()));
        transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(5)));

        // Set stream limits
        transport_config.max_concurrent_bidi_streams(100u32.into());
        transport_config.max_concurrent_uni_streams(100u32.into());

        server_config.transport_config(Arc::new(transport_config));

        Ok(server_config)
    }

    /// Create client configuration (accepts any certificate)
    fn create_client_config() -> Result<quinn::ClientConfig, EndpointError> {
        // Skip certificate verification for P2P network
        // In production, we'd verify based on NodeID
        let crypto = rustls::ClientConfig::builder_with_provider(
                Arc::new(rustls::crypto::ring::default_provider())
            )
            .with_safe_default_protocol_versions()
            .map_err(|e| EndpointError::ConfigCreation(format!("Failed to set protocol versions: {:?}", e)))?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        let mut client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
                .map_err(|e| EndpointError::ConfigCreation(format!("Failed to create QUIC client config: {:?}", e)))?
        ));

        // Configure transport
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into().unwrap()));
        transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(5)));
        transport_config.max_concurrent_bidi_streams(100u32.into());
        transport_config.max_concurrent_uni_streams(100u32.into());

        client_config.transport_config(Arc::new(transport_config));

        Ok(client_config)
    }

    /// Connect to a remote endpoint
    pub async fn connect(&self, addr: SocketAddr) -> Result<super::Connection, EndpointError> {
        let connecting = self.inner
            .connect(addr, "anonnet.local")
            .map_err(|e| EndpointError::ConnectionFailed(e.to_string()))?;

        let connection = connecting.await
            .map_err(|e| EndpointError::ConnectionFailed(e.to_string()))?;

        Ok(super::Connection::new(connection))
    }

    /// Accept an incoming connection
    pub async fn accept(&self) -> Result<super::Connection, EndpointError> {
        let connecting = self.inner
            .accept()
            .await
            .ok_or(EndpointError::Closed)?;

        let connection = connecting.await
            .map_err(|e| EndpointError::ConnectionFailed(e.to_string()))?;

        Ok(super::Connection::new(connection))
    }

    /// Get local address
    /// If bound to 0.0.0.0, returns 127.0.0.1 instead for local connections
    pub fn local_addr(&self) -> SocketAddr {
        let mut addr = self.local_addr;
        if addr.ip().is_unspecified() {
            addr.set_ip(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
        }
        addr
    }

    /// Close the endpoint
    pub fn close(&self) {
        self.inner.close(0u32.into(), b"shutdown");
    }
}

/// Endpoint configuration
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    /// Address to bind to
    pub bind_addr: SocketAddr,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:0".parse().unwrap(),
        }
    }
}

impl EndpointConfig {
    /// Create configuration with specific bind address
    pub fn with_bind_addr(bind_addr: SocketAddr) -> Self {
        Self { bind_addr }
    }
}

/// Skip certificate verification for P2P network
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        // Skip verification - in production, verify based on NodeID
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

/// Endpoint errors
#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    #[error("Failed to bind to address: {0}")]
    BindFailed(String),

    #[error("Failed to generate certificate: {0}")]
    CertGeneration(String),

    #[error("Failed to create config: {0}")]
    ConfigCreation(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Endpoint is closed")]
    Closed,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_endpoint_creation() {
        let config = EndpointConfig::default();
        let endpoint = Endpoint::new(config).await.unwrap();

        assert_ne!(endpoint.local_addr().port(), 0);
    }

    #[tokio::test]
    async fn test_endpoint_connect() {
        // Create server endpoint
        let server_config = EndpointConfig::default();
        let server = Endpoint::new(server_config).await.unwrap();
        let server_addr = server.local_addr();

        // Create client endpoint
        let client_config = EndpointConfig::default();
        let client = Endpoint::new(client_config).await.unwrap();

        // Spawn server accept task
        let accept_task = tokio::spawn(async move {
            server.accept().await
        });

        // Client connects
        let client_conn = client.connect(server_addr).await.unwrap();

        // Server accepts
        let server_conn = accept_task.await.unwrap().unwrap();

        // Both connections should be established
        assert_eq!(client_conn.remote_addr(), server_addr);
        assert_eq!(server_conn.remote_addr(), client.local_addr());
    }
}
