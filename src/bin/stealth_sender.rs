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
    println!("--- PROTEUS STEALTH BEACON (SINGLE LINE MODE) ---");

    // 1. Load Secrets
    dotenv().ok();
    let target_ip = env::var("TARGET_IP").expect("Check .env file!");
    println!("[READY] Sending to [REDACTED]");

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind");

    // 2. The Payload
    // We make it longer to prove it handles data
    let plaintext = b"PROTEUS FINAL: We are live. The network is open. Stealth mode active.";
    
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

        // 3. Cloak as ONE-LINE HTTP (Safe for Phone Loggers)
        let b64_data = general_purpose::STANDARD.encode(&data);

        // Looks like: GET /search?q=AbCd123... HTTP/1.1
        let http_packet = format!(
            "GET /search?q={}&seq={} HTTP/1.1", 
            b64_data, seq
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

        thread::sleep(Duration::from_millis(100));
    }
}