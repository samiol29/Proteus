use std::net::{UdpSocket, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use raptorq::Encoder;
use base64::{Engine as _, engine::general_purpose};
use chacha20poly1305::{aead::{Aead, AeadCore, KeyInit, OsRng}, XChaCha20Poly1305};
use crate::{SYMBOL_SIZE, framing, oracle, transport};

pub fn start_sender(target: String, message: String, use_tcp: bool) {
    println!("[CLIENT] Target: {} | Mode: {}", target, if use_tcp { "SHADOW TCP" } else { "UDP" });
    
    let transport = if use_tcp {
        println!("[SETUP] Connecting TCP...");
        let stream = TcpStream::connect(&target).expect("TCP Failed");
        stream.set_nodelay(true).ok();
        // FIXED: Added Arc::new(...) to match the new TransportType
        transport::TransportType::Tcp(Arc::new(Mutex::new(stream)))
    } else {
        let socket = UdpSocket::bind("0.0.0.0:0").expect("UDP Bind Failed");
        transport::TransportType::Udp(socket)
    };

    let oracle = Arc::new(Mutex::new(oracle::NetworkOracle::new()));
    
    let key_bytes = [0u8; 32];
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted = cipher.encrypt(&nonce, message.as_bytes()).unwrap();
    
    let mut blob = nonce.to_vec();
    blob.extend(encrypted);
    let length = blob.len() as u16;
    let mut final_payload = length.to_be_bytes().to_vec();
    final_payload.extend(blob);

    let encoder = Encoder::with_defaults(&final_payload, SYMBOL_SIZE);

    println!("[CLIENT] Sending...");
    let mut seq = 0;
    loop {
        let packets = encoder.get_encoded_packets(1);
        let symbol_data = packets[0].serialize();

        let header = framing::PacketHeader::new(seq);
        let header_bytes = header.to_bytes();
        let mut packet_data = header_bytes.to_vec();
        packet_data.extend(symbol_data);

        let b64_data = general_purpose::STANDARD.encode(&packet_data);
        // Important: Add newline for compatibility with the new server
        let http_packet = format!("GET /search?q={}&seq={} HTTP/1.1\n", b64_data, seq);

        transport.send(http_packet.as_bytes(), &target).ok();

        let pacing;
        {
            // Now we can use the updated public methods
            let brain = oracle.lock().unwrap();
            pacing = brain.get_pacing_interval();
        }
        seq += 1;
        thread::sleep(pacing);
    }
}