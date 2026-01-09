use std::io::Read;
use std::net::TcpStream;
use proteus_core::SYMBOL_SIZE;
use raptorq::{Decoder, ObjectTransmissionInformation, EncodingPacket};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce
};

fn main() {
    println!("--- PROTEUS CLIENT v2 (DYNAMIC HANDSHAKE) ---");
    println!("Connecting to 10.0.0.2:80...");

    let mut stream = TcpStream::connect("10.0.0.2:80")
        .expect("Could not connect. Is the node running?");

    println!("[CONNECTED] Waiting for Handshake...");

    // --- STEP 1: READ THE HANDSHAKE ---
    // We expect exactly 8 bytes (u64) defining the file size
    let mut size_buffer = [0u8; 8];
    stream.read_exact(&mut size_buffer).expect("Failed to read handshake!");
    
    let total_blob_size = u64::from_be_bytes(size_buffer);
    println!("[HANDSHAKE] Server says payload is: {} bytes", total_blob_size);

    // --- STEP 2: CONFIGURE DECODER DYNAMICALLY ---
    let raptor_packet_len = 4 + SYMBOL_SIZE as usize; 
    let total_frame_len = 5 + raptor_packet_len; // PROT: + ID + Data
    
    let config = ObjectTransmissionInformation::new(
        total_blob_size, // <--- NO MORE HARDCODED NUMBERS!
        SYMBOL_SIZE, 
        1, 1, 1
    );
    let mut decoder = Decoder::new(config);

    // Crypto
    let key_bytes = [0u8; 32];
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());

    let mut buffer = [0u8; 4096];
    let mut accumulator = Vec::new();

    println!("[STREAM] Receiving symbols...");

    // --- STEP 3: MAIN LOOP ---
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                accumulator.extend_from_slice(&buffer[..n]);

                while let Some(pos) = find_subsequence(&accumulator, b"PROT:") {
                    if accumulator.len() >= pos + total_frame_len {
                        
                        let packet_start = pos + 5;
                        let packet_end = pos + total_frame_len;
                        let packet_bytes = &accumulator[packet_start..packet_end];

                        let packet = EncodingPacket::deserialize(packet_bytes);
                        
                        if let Some(decoded_data) = decoder.decode(packet) {
                            println!("\n\n[!!!] RESURRECTION COMPLETE!");
                            
                            let (nonce_bytes, ciphertext) = decoded_data.split_at(24);
                            let nonce = XNonce::from_slice(nonce_bytes);
                            
                            match cipher.decrypt(nonce, ciphertext) {
                                Ok(msg) => {
                                    // Clean null padding
                                    let clean_msg = msg.iter().take_while(|&&x| x != 0).cloned().collect::<Vec<u8>>();
                                    println!("------------------------------------------------");
                                    println!("MESSAGE: \"{}\"", String::from_utf8_lossy(&clean_msg));
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

                        accumulator.drain(0..packet_end);
                    } else {
                        break;
                    }
                }
            },
            Err(e) => panic!("Connection Error: {}", e),
        }
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}