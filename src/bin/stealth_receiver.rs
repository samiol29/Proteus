use std::net::UdpSocket;
use std::convert::TryInto;
use raptorq::{Decoder, ObjectTransmissionInformation, EncodingPacket};
use base64::{Engine as _, engine::general_purpose};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce
};
use proteus_core::{SYMBOL_SIZE, framing};

fn main() {
    println!("--- PROTEUS STEALTH RECEIVER (PROTOCOL V1) ---");
    println!("Listening for 'Google Search' traffic on Port 9000...");

    let socket = UdpSocket::bind("0.0.0.0:9000").expect("Could not bind to port 9000");

    let approx_payload_size = 512; 
    let config = ObjectTransmissionInformation::new(
        approx_payload_size, 
        SYMBOL_SIZE, 
        1, 1, 1
    );
    let mut decoder = Decoder::new(config);

    let key_bytes = [0u8; 32];
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());

    let mut buffer = [0u8; 2048];

    loop {
        // [FIXED] We name the source address 'src' (no underscore) so we can use it
        match socket.recv_from(&mut buffer) {
            Ok((size, src)) => {
                let msg_str = String::from_utf8_lossy(&buffer[..size]);

                if let Some(start) = msg_str.find("q=") {
                    if let Some(end) = msg_str.find("&seq") {
                        let b64_data = &msg_str[start+2..end];
                        
                        if let Ok(binary_data) = general_purpose::STANDARD.decode(b64_data) {
                            
                            // [LAYER 1] PARSE PROTEUS HEADER
                            if binary_data.len() < framing::HEADER_SIZE { continue; }
                            
                            let (head_bytes, symbol_bytes) = binary_data.split_at(framing::HEADER_SIZE);
                            
                            if let Some(header) = framing::PacketHeader::from_bytes(head_bytes) {
                                // [FIXED] SEND ACK
                                // We use 'src' here, which matches the variable above
                                let ack = framing::AckPacket::new(header.seq_id, header.timestamp);
                                let ack_bytes = ack.to_bytes();
                                socket.send_to(&ack_bytes, src).ok();
                            }

                            // [LAYER 2] RAPTORQ & DECRYPT
                            let packet = EncodingPacket::deserialize(&symbol_bytes.to_vec());
                            
                            if let Some(decoded_data) = decoder.decode(packet) {
                                println!("\n[!!!] RESURRECTION COMPLETE!");

                                if decoded_data.len() < 2 { continue; } 
                                
                                let len_bytes: [u8; 2] = decoded_data[0..2].try_into().unwrap();
                                let real_len = u16::from_be_bytes(len_bytes) as usize;

                                println!("-> Size Header says: {} bytes (Buffer is {})", real_len, decoded_data.len());

                                if decoded_data.len() < 2 + real_len { continue; }
                                let valid_data = &decoded_data[2..2+real_len];

                                let (nonce_bytes, ciphertext) = valid_data.split_at(24);
                                let nonce = XNonce::from_slice(nonce_bytes);
                                
                                match cipher.decrypt(nonce, ciphertext) {
                                    Ok(msg) => {
                                        println!("------------------------------------------------");
                                        println!("MESSAGE: \"{}\"", String::from_utf8_lossy(&msg));
                                        println!("------------------------------------------------");
                                        return;
                                    },
                                    Err(_) => println!("Decryption Error"),
                                }
                            } else {
                                print!("."); 
                                use std::io::Write;
                                std::io::stdout().flush().unwrap();
                            }
                        }
                    }
                }
            },
            Err(e) => println!("Rx Error: {}", e),
        }
    }
}