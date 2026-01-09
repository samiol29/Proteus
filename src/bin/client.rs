use tokio::net::UdpSocket;
use bincode;
use proteus_core::{ProteusPacket, SERVER_ADDR, SYMBOL_SIZE}; // FIX: Use SYMBOL_SIZE
use raptorq::Encoder;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- PROTEUS CLIENT (SENDER) ---");

    // 1. Prepare Data
    let plaintext = b"PROTEUS PHASE 2: This message traveled over real UDP packets!";
    
    // 2. Encrypt
    let key_bytes = [0u8; 32];
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted = cipher.encrypt(&nonce, plaintext.as_ref()).unwrap();
    
    let mut blob = nonce.to_vec();
    blob.extend(encrypted);

    // 3. Encode
    // FIX: Use the exact same SYMBOL_SIZE as the server
    let encoder = Encoder::with_defaults(&blob, SYMBOL_SIZE);
    
    // Generate packets
    let packets = encoder.get_encoded_packets(1000); 

    // 4. Connect
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(SERVER_ADDR).await?;
    
    println!("Sending {} packets to {}...", packets.len(), SERVER_ADDR);
    
    // Buffer for feedback
    let mut buf = [0u8; 2048];

    for (i, raptor_packet) in packets.into_iter().enumerate() {
        let payload_bytes = raptor_packet.serialize();

        let protocol_packet = ProteusPacket::Data {
            seq: i as u32,
            payload: payload_bytes,
        };
        let bytes = bincode::serialize(&protocol_packet)?;

        // Send
        socket.send(&bytes).await?;
        print!(">"); 

        // Check Feedback (Don't crash if connection refused initially)
        if let Ok(len) = socket.try_recv(&mut buf) {
            if let Ok(ProteusPacket::Control { is_complete, .. }) = bincode::deserialize(&buf[..len]) {
                if is_complete {
                    println!("\n[√] Server signaled completion! Stopping.");
                    break;
                }
            }
        }
        
        // Slow down slightly to see the progress
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    Ok(())
}