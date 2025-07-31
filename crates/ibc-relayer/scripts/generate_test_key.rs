#!/usr/bin/env rust-script

//! Generate a test Cosmos key for testnet deployment
//! 
//! ```cargo
//! [dependencies]
//! secp256k1 = "0.28"
//! rand = "0.8"
//! sha2 = "0.10"
//! bech32 = "0.9"
//! hex = "0.4"
//! ```

use secp256k1::{Secp256k1, SecretKey, PublicKey, All};
use sha2::{Digest, Sha256};
use rand::rngs::OsRng;

fn main() {
    println!("🔑 Generating test Cosmos key for provider testnet...");
    
    // Generate random private key
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut OsRng);
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    
    // Get private key bytes
    let private_key_bytes = secret_key.secret_bytes();
    let private_key_hex = hex::encode(private_key_bytes);
    
    // Get public key
    let public_key_bytes = public_key.serialize();
    let public_key_hex = hex::encode(public_key_bytes);
    
    // Derive address - use compressed public key
    let public_key_hash = Sha256::digest(&public_key.serialize_uncompressed()[1..65]);
    let address_bytes = &public_key_hash[0..20];
    
    // Convert to bech32 5-bit encoding
    let address_5bit = bech32::convert_bits(address_bytes, 8, 5, true).unwrap();
    let address = bech32::encode("cosmos", address_5bit, bech32::Variant::Bech32).unwrap();
    
    println!("✅ Generated Cosmos key:");
    println!("   Private Key: {}", private_key_hex);
    println!("   Public Key:  {}", public_key_hex);
    println!("   Address:     {}", address);
    println!("");
    println!("💰 To fund this address, visit:");
    println!("   https://faucet.cosmoskit.com/ (if available)");
    println!("   Or ask in Cosmos Discord for testnet tokens");
    println!("");
    println!("🔐 To add to keystore, run:");
    println!("   cargo run --bin key-manager add provider --key-type cosmos");
    println!("   Then enter the private key and address above when prompted");
}