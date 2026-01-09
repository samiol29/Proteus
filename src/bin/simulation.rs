use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce
};
use raptorq::{Encoder, Decoder, EncodingPacket, ObjectTransmissionInformation};
use rand::Rng;
use std::time::Instant;

fn main() {
    println!("--- PROTEUS CORE [PHASE 1]: SECURE RESURRECTION (FIXED) ---");

    // 1. THE PAYLOAD
    let original_plaintext = b"PROTEUS_SECRET: The tank is moving. Maintain velocity at 40 knots.";
    println!("\n[1] ORIGINAL DATA: {} bytes", original_plaintext.len());

    // 2. ENCRYPTION (XChaCha20-Poly1305)
    println!("[2] ENCRYPTING...");
    let key = XChaCha20Poly1305::generate_key(&mut OsRng);
    let cipher = XChaCha20Poly1305::new(&key);
    // XChaCha uses a 24-byte Nonce (XNonce), not the standard 12-byte one
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    
    let encrypted_data = cipher.encrypt(&nonce, original_plaintext.as_ref())
        .expect("Encryption failed!");
    
    // Bundle Nonce + Ciphertext
    let mut transmission_blob = nonce.to_vec();
    transmission_blob.extend(&encrypted_data);
    println!("    > Encrypted Blob Size: {} bytes", transmission_blob.len());

    // 3. ENCODING (RaptorQ High-Level API)
    println!("[3] ENCODING...");
    let start_enc = Instant::now();
    let packet_size = 4; // Tiny size to force many packets for the demo
    
    // Create the Encoder with the data
    let encoder = Encoder::with_defaults(&transmission_blob, packet_size as u16);
    
    // Generate Source Packets + Repair Packets
    // We calculate how many source packets we have, then ask for 50% more (Repair)
    let source_count = (transmission_blob.len() as f32 / packet_size as f32).ceil() as u32;
    let repair_count = (source_count as f32 * 0.5) as u32;
    let total_packets_needed = source_count + repair_count;

    // Get the packets
    let packets = encoder.get_encoded_packets(total_packets_needed);

    println!("    > Source Blocks: {}", source_count);
    println!("    > Total Packets Generated: {}", packets.len());
    println!("    > Encoding Time: {:?}", start_enc.elapsed());

    // 4. SIMULATION (The Firewall)
    println!("\n[4] SIMULATING 40% LOSS...");
    let mut received_packets: Vec<EncodingPacket> = Vec::new();
    let mut rng = rand::rng(); 
    let mut lost_count = 0;

    for packet in packets {
        // 40% chance to drop the packet
        if rng.random_bool(0.4) {
            print!("x");
            lost_count += 1;
        } else {
            print!(".");
            received_packets.push(packet);
        }
    }
    println!("\n    > Lost: {}", lost_count);
    println!("    > Received: {}", received_packets.len());

    // 5. DECODING
    println!("\n[5] DECODING...");
    let start_dec = Instant::now();
    
    // Reconstruct the Config (Receiver needs to know total size and packet size)
    let config = ObjectTransmissionInformation::new(
        transmission_blob.len() as u64,
        packet_size as u16, 
        1, 1, 1
    );
    let mut decoder = Decoder::new(config);
    let mut result: Option<Vec<u8>> = None;

    // Feed packets into the mathematical engine
    for packet in received_packets {
        // .decode() returns Some(data) immediately when it has enough symbols
        if let Some(data) = decoder.decode(packet) {
            result = Some(data);
            break; // Stop decoding, we have the file!
        }
    }

    match result {
        Some(data) => {
            println!("    > Resurrection Successful!");
            println!("    > Decoding Time: {:?}", start_dec.elapsed());
            
            // 6. DECRYPTION
            println!("\n[6] DECRYPTING...");
            // Split Nonce (24 bytes) and Ciphertext
            let (nonce_bytes, ciphertext) = data.split_at(24);
            let nonce = XNonce::from_slice(nonce_bytes);
            
            let decrypted = cipher.decrypt(nonce, ciphertext)
                .expect("Decryption failed! Integrity Check Failed.");
                
            println!("SUCCESS: \"{}\"", String::from_utf8(decrypted).unwrap());
        },
        None => println!("FAILURE: Not enough packets survived. Increase Repair Overhead."),
    }
}