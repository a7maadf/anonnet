/// Generate a .anon service address
///
/// This example demonstrates how to create a cryptographic .anon address
/// similar to Tor's .onion addresses.

use anonnet_core::identity::KeyPair;
use anonnet_core::service::ServiceAddress;

fn main() {
    println!("🌐 AnonNet .anon Address Generator");
    println!("=====================================\n");

    // Generate a new keypair for the service
    println!("🔑 Generating Ed25519 keypair for service...");
    let keypair = KeyPair::generate();
    let public_key = keypair.public_key();

    println!("   ✅ Keypair generated\n");

    // Derive .anon address from public key
    println!("📝 Deriving .anon address from public key...");
    let service_addr = ServiceAddress::from_public_key(&public_key);

    println!("   Algorithm: BLAKE3 hash");
    println!("   Domain: 'ANONNET-SERVICE-V1'");
    println!("   Encoding: Base32 (lowercase, no padding)\n");

    // Display the address
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ Your .anon Address:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("   {}\n", service_addr.to_hostname());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Show details
    println!("📊 Address Details:");
    println!("   Base32: {}", service_addr.to_base32());
    println!("   Length: {} characters", service_addr.to_base32().len());
    println!("   Full:   {} characters", service_addr.to_hostname().len());
    println!();

    // Show public key
    println!("🔐 Service Public Key (hex):");
    let pub_key_bytes = public_key.as_bytes();
    print!("   ");
    for (i, byte) in pub_key_bytes.iter().enumerate() {
        print!("{:02x}", byte);
        if (i + 1) % 16 == 0 && i < 31 {
            print!("\n   ");
        }
    }
    println!("\n");

    // Verify the address
    println!("🔍 Verification:");
    if service_addr.verify_public_key(&public_key) {
        println!("   ✅ Address correctly derived from public key");
    } else {
        println!("   ❌ Address verification failed!");
    }
    println!();

    // Show how clients would verify
    println!("💡 Security Properties:");
    println!("   • Address is cryptographically bound to public key");
    println!("   • Cannot be forged without the private key");
    println!("   • Clients can verify they're talking to the right service");
    println!("   • Similar security model to Tor .onion addresses");
    println!();

    println!("🎯 Next Steps:");
    println!("   1. Save this keypair to start your service");
    println!("   2. Create a service descriptor with introduction points");
    println!("   3. Sign the descriptor with the private key");
    println!("   4. Publish descriptor to the DHT");
    println!("   5. Clients can now discover your service!");
    println!();

    println!("✨ Address generation complete!");
}
