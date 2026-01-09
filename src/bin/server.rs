use tokio::net::UdpSocket;
use bincode;
use proteus_core::{ProteusPacket, SERVER_ADDR, SYMBOL_SIZE}; // FIX: Use SYMBOL_SIZE
use raptorq::{Decoder, ObjectTransmissionInformation, EncodingPacket};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- PROTEUS SERVER (RECEIVER) ---");
    println!("Listening on {}...", SERVER_ADDR);

    // 1. Bind to UDP Port
    let socket = UdpSocket::bind(SERVER_ADDR).await?;
    let mut buf = [0u8; 2048]; 

    // 2. Setup Decoder
    // In production, the first packet sends the file size.
    // Here we hardcode the size of the known message (~100 bytes).
    let original_data_size = 150; 
    let config = ObjectTransmissionInformation::new(
        original_data_size, 
        SYMBOL_SIZE, // FIX: Strictly use the shared constant
        1, 1, 1
    );
    let mut decoder = Decoder::new(config);

    // Crypto
    let key_bytes = [0u8; 32];
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());

    println!("Waiting for packets...");

    loop {
        // 3. Receive
        let (len, addr) = socket.recv_from(&mut buf).await?;
        
        // 4. Deserialize
        // We handle errors gracefully so the server doesn't crash on bad packets
        match bincode::deserialize::<ProteusPacket>(&buf[..len]) {
            Ok(ProteusPacket::Data { seq: _, payload }) => {
                print!("."); 

                // Decode
                let encoding_packet = EncodingPacket::deserialize(&payload);
                let result = decoder.decode(encoding_packet);

                // Send Feedback
                let feedback = ProteusPacket::Control {
                    current_rank: 0, 
                    is_complete: result.is_some(),
                };
                let feedback_bytes = bincode::serialize(&feedback)?;
                socket.send_to(&feedback_bytes, addr).await?;

                // Check Victory
                if let Some(data) = result {
                    println!("\n\n[!!!] RESURRECTION COMPLETE!");
                    
                    let (nonce_bytes, ciphertext) = data.split_at(24);
                    let nonce = XNonce::from_slice(nonce_bytes);
                    
                    if let Ok(msg) = cipher.decrypt(nonce, ciphertext) {
                         println!("DECRYPTED MESSAGE: \"{}\"", String::from_utf8_lossy(&msg));
                    }
                    break;
                }
            },
            Ok(_) => {}, // Ignore Control packets sent to Server
            Err(_) => { /* Ignore garbage bytes */ }
        }
    }
    Ok(())
}