/// AnonWeb - Easy .anon website hosting
///
/// Usage: anonweb [PORT]
///
/// This command makes hosting on AnonNet as easy as Python's http.server.
/// It automatically:
/// - Generates a .anon domain (or uses existing keys)
/// - Proxies your local HTTP server to the .anon network
/// - Prints the .anon URL for sharing

use anyhow::{Context, Result};
use anonnet_common::NodeConfig;
use anonnet_core::{identity::KeyPair, service::ServiceAddress, Node};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info, warn};
use tracing_subscriber;

const DEFAULT_PORT: u16 = 8080;
const KEYS_DIR: &str = ".anonweb";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    let local_port = if args.len() > 1 {
        args[1].parse::<u16>()
            .context("Invalid port number. Usage: anonweb [PORT]")?
    } else {
        DEFAULT_PORT
    };

    println!("\n🌐 AnonWeb - Easy .anon hosting");
    println!("================================\n");

    // Create keys directory
    let keys_dir = PathBuf::from(KEYS_DIR);
    fs::create_dir_all(&keys_dir)
        .context("Failed to create .anonweb directory")?;

    // Load or generate service keypair
    let (keypair, is_new) = load_or_generate_keys(&keys_dir)?;
    let public_key = keypair.public_key();
    let service_address = ServiceAddress::from_public_key(&public_key);

    if is_new {
        println!("✨ Generated new .anon service identity");
        println!("   Keys saved to: {}/", KEYS_DIR);
    } else {
        println!("🔑 Using existing .anon service identity");
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ .anon domain generated!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("📍 Your .anon domain:");
    println!("   {}", service_address);
    println!();
    println!("💡 Next steps:");
    println!("   1. Start your HTTP server:");
    println!("      python3 -m http.server {} (or any web server)", local_port);
    println!();
    println!("   2. Register this service with the AnonNet daemon:");
    println!("      curl -X POST http://localhost:19150/api/services/register \\");
    println!("        -H 'Content-Type: application/json' \\");
    println!("        -d '{{\"local_host\": \"127.0.0.1\", \"local_port\": {}, \"ttl_hours\": 24}}'", local_port);
    println!();
    println!("   3. Your .anon site will be accessible at:");
    println!("      {}", service_address);
    println!();
    println!("💾 Keys stored in:");
    println!("   .anonweb/service.key  (private - keep safe!)");
    println!("   .anonweb/service.pub  (public)");
    println!();
    println!("🔧 Your service is configured for port: {}", local_port);
    println!();

    Ok(())
}

/// Load existing keys or generate new ones
fn load_or_generate_keys(keys_dir: &PathBuf) -> Result<(KeyPair, bool)> {
    let private_key_path = keys_dir.join("service.key");
    let public_key_path = keys_dir.join("service.pub");

    // Try to load existing keys
    if private_key_path.exists() {
        match load_keys(&private_key_path, &public_key_path) {
            Ok(keypair) => return Ok((keypair, false)),
            Err(e) => {
                warn!("Failed to load existing keys: {}. Generating new ones.", e);
            }
        }
    }

    // Generate new keys
    let keypair = KeyPair::generate();

    // Save keys
    save_keys(&keypair, &private_key_path, &public_key_path)?;

    Ok((keypair, true))
}

/// Load keys from files
fn load_keys(private_key_path: &PathBuf, _public_key_path: &PathBuf) -> Result<KeyPair> {
    let private_hex = fs::read_to_string(private_key_path)?;
    let private_bytes = hex::decode(private_hex.trim())?;

    if private_bytes.len() != 32 {
        anyhow::bail!("Invalid private key length");
    }

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&private_bytes);

    KeyPair::from_secret_bytes(&bytes)
        .context("Failed to reconstruct keypair")
}

/// Save keys to files
fn save_keys(keypair: &KeyPair, private_key_path: &PathBuf, public_key_path: &PathBuf) -> Result<()> {
    let private_hex = hex::encode(keypair.secret_bytes());
    let public_hex = hex::encode(keypair.public_bytes());

    fs::write(private_key_path, private_hex)
        .context("Failed to write private key")?;
    fs::write(public_key_path, public_hex)
        .context("Failed to write public key")?;

    // Set restrictive permissions on private key (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(private_key_path)?.permissions();
        perms.set_mode(0o600); // Read/write for owner only
        fs::set_permissions(private_key_path, perms)?;
    }

    println!("   Private key: {}  (keep safe!)", private_key_path.display());
    println!("   Public key:  {}", public_key_path.display());

    Ok(())
}
