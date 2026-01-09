use std::net::UdpSocket;
use std::thread;
use std::time::Duration;
use raptorq::Encoder;
use base64::{Engine as _, engine::general_purpose};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305
};
use proteus_core::SYMBOL_SIZE;
use dotenv::dotenv;
use std::env;

fn main() {
    println!("--- PROTEUS STEALTH BEACON (HTTP MODE) ---");

    // 1. Load Secrets securely
    dotenv().ok();
    let target_ip = env::var("TARGET_IP")
        .expect("ERROR: TARGET_IP not set in .env file!");

    // CHANGED: We now hide the IP in the console output
    println!("[READY] Mimicking Google Traffic to [REDACTED TARGET]");

    // Bind to all interfaces
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Could not bind socket");

    // 2. The Payload
    let plaintext = b"PROTEUS STEALTH: This message is hidden inside a fake Google HTTP request.";
    
    // Encrypt
    let key_bytes = [0u8; 32];
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted = cipher.encrypt(&nonce, plaintext.as_ref()).unwrap();
    
    let mut blob = nonce.to_vec();
    blob.extend(encrypted);

    let encoder = Encoder::with_defaults(&blob, SYMBOL_SIZE);

    let mut seq = 0;
    loop {
        let packets = encoder.get_encoded_packets(1);
        let symbol = &packets[0];
        let data = symbol.serialize();

        // 3. Cloak as HTTP
        let b64_data = general_purpose::STANDARD.encode(&data);

        let http_packet = format!(
            "GET /api/v1/sync?seq={} HTTP/1.1\r\n\
             Host: www.google.com\r\n\
             User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64)\r\n\
             X-Goog-Payload: {}\r\n\
             \r\n", 
            seq, b64_data
        );

        match socket.send_to(http_packet.as_bytes(), &target_ip) {
            Ok(_) => {
                seq += 1;
                if seq % 10 == 0 { print!("."); }
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            },
            Err(e) => println!("Tx Error: {}", e),
        }

        thread::sleep(Duration::from_millis(50));
    }
}