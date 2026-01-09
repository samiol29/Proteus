use clap::{Parser, Subcommand};
use proteus_core::{vpn, transport, oracle, SYMBOL_SIZE, framing};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader, Write, Read};
use std::convert::TryInto;
use std::time::Instant;
use std::thread; // Needed for server threads
use raptorq::{Encoder, Decoder, ObjectTransmissionInformation, EncodingPacket};
use base64::{Engine as _, engine::general_purpose};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce
};

const PACKET_TARGET_SIZE: usize = 400; 

#[derive(Parser)]
#[command(name = "Proteus")]
#[command(version = "1.0.0")]
#[command(about = "Proteus: The Unbreakable Tank (Production)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Send { target: String, #[arg(short, long)] message: String, #[arg(long, action)] tcp: bool },
    Recv { #[arg(short, long, default_value_t = 9000)] port: u16 },
    Vpn { target: String },
    Relay { #[arg(short, long, default_value_t = 9000)] port: u16 },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Send { target, message, tcp } => proteus_core::client::start_sender(target.clone(), message.clone(), *tcp),
        Commands::Recv { .. } => println!("Use 'proteus relay' instead."),
        Commands::Vpn { target } => run_smart_client(target.clone()), 
        Commands::Relay { port } => run_relay_server(*port),
    }
}

// --- CLIENT (TANK) ---
fn run_smart_client(target: String) {
    println!("--- PROTEUS TANK CLIENT ---");
    let vpn = vpn::ProteusVpn::new();
    let mut brain = oracle::NetworkOracle::new();
    println!("[BRAIN] Oracle Online. Learning Network Dynamics...");

    let stream = TcpStream::connect(&target).expect("Connection Failed");
    stream.set_nodelay(true).ok();
    stream.set_nonblocking(true).ok();
    
    let transport_stream = Arc::new(Mutex::new(stream));
    let transport = transport::TransportType::Tcp(transport_stream.clone());
    let key_bytes = [0u8; 32];
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());

    let mut buf = [0u8; 1500];
    let mut seq = 0;
    let mut last_feedback = Instant::now();

    loop {
        // 1. READ FEEDBACK / INCOMING DATA (From Server)
        {
            let mut conn = transport_stream.lock().unwrap();
            let mut inc_buf = [0u8; 2048]; // Bigger buffer for incoming downloads
            match conn.read(&mut inc_buf) {
                Ok(n) if n > 0 => {
                    let data = &inc_buf[..n];
                    // If it's the simple "OK" ACK
                    if data == b"OK" {
                         let now = Instant::now();
                         brain.update(now.duration_since(last_feedback));
                         last_feedback = now;
                    } else {
                        // IT IS DATA FROM THE INTERNET!
                        // In a real full impl, we would decrypt this too.
                        // For this demo, we assume the downlink is raw for speed/simplicity
                        // or we just dump it to the TUN.
                        vpn.write(data).ok();
                    }
                },
                _ => {}
            }
        }

        // 2. READ KERNEL (Outgoing)
        match vpn.read(&mut buf) {
            Ok(size) if size > 0 => {
                let packet_data = &buf[..size];
                let loss_ratio = brain.loss_rate; 
                let redundant_packets = if loss_ratio > 0.1 { 2 } else { 1 };
                
                if seq % 50 == 0 {
                   // Only log occasionally to keep terminal clean
                   println!("[STATUS] Loss: {:.2} | RTT: {:?} | Redundancy: {}x", loss_ratio, brain.smoothed_rtt, redundant_packets);
                }

                let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
                let encrypted = cipher.encrypt(&nonce, packet_data).unwrap();
                let mut blob = nonce.to_vec();
                blob.extend(encrypted);
                let length = blob.len() as u16;
                let mut final_payload = length.to_be_bytes().to_vec();
                final_payload.extend(blob);
                
                if final_payload.len() < PACKET_TARGET_SIZE {
                    let padding = PACKET_TARGET_SIZE - final_payload.len();
                    final_payload.extend(std::iter::repeat(0).take(padding));
                }

                let encoder = Encoder::with_defaults(&final_payload, SYMBOL_SIZE);
                let packets = encoder.get_encoded_packets(redundant_packets);

                for packet in packets {
                    let symbol_data = packet.serialize();
                    let header = framing::PacketHeader::new(seq);
                    let header_bytes = header.to_bytes();
                    let mut wire_data = header_bytes.to_vec();
                    wire_data.extend(symbol_data);

                    let b64_data = general_purpose::STANDARD.encode(&wire_data);
                    let http_packet = format!("GET /search?q={}&seq={} HTTP/1.1\n", b64_data, seq);

                    if let Err(_) = transport.send(http_packet.as_bytes(), &target) { break; }
                }
                seq += 1;
            },
            Ok(_) => {}, 
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(brain.get_pacing_interval());
            },
            Err(e) => println!("TUN Error: {}", e),
        }
    }
}

// --- SERVER (GATEWAY) ---
fn run_relay_server(port: u16) {
    println!("--- PROTEUS GATEWAY SERVER ---");
    
    // 1. OPEN TUN DEVICE (Shared System Interface)
    let vpn_dev = Arc::new(vpn::ProteusVpn::new());
    
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind");
    let key_bytes = [0u8; 32];
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());
    let config = ObjectTransmissionInformation::new(PACKET_TARGET_SIZE as u64, SYMBOL_SIZE as u16, 1, 1, 1);
    
    println!("[LISTENING] Gateway Active on Port {}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(socket) => {
                println!("[NEW TANK CONNECTED] {:?}", socket.peer_addr());
                
                // CLONE RESOURCES FOR THREADS
                let socket_reader = socket.try_clone().expect("Clone failed");
                let mut socket_writer = socket.try_clone().expect("Clone failed");
                let vpn_writer = vpn_dev.clone();
                let vpn_reader = vpn_dev.clone();
                
                // THREAD 1: UPLINK (Internet -> Client)
                // Reads from TUN (responses) and writes to TCP
                thread::spawn(move || {
                    let mut buf = [0u8; 1500];
                    loop {
                        match vpn_reader.read(&mut buf) {
                            Ok(n) if n > 0 => {
                                // We caught a packet from Google meant for the Client!
                                // For MVP: Send Raw. (Production: Encrypt this too)
                                if let Err(_) = socket_writer.write_all(&buf[..n]) {
                                    break;
                                }
                            },
                            Ok(_) => {},
                            Err(_) => std::thread::sleep(std::time::Duration::from_millis(1)),
                        }
                    }
                });

                // MAIN LOOP: DOWNLINK (Client -> Internet)
                // Reads from TCP, Decrypts, Writes to TUN
                let cipher_clone = XChaCha20Poly1305::new(&key_bytes.into());
                let mut reader = BufReader::new(socket_reader);
                let mut line = String::new();

                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break, 
                        Ok(_) => {
                            if let Some(start) = line.find("q=") {
                                if let Some(end) = line.find("&seq") {
                                    let b64 = &line[start+2..end];
                                    if let Ok(wire_bytes) = general_purpose::STANDARD.decode(b64) {
                                        if wire_bytes.len() < framing::HEADER_SIZE { continue; }
                                        let (_head, symbol) = wire_bytes.split_at(framing::HEADER_SIZE);
                                        
                                        let mut decoder = Decoder::new(config);
                                        let packet = EncodingPacket::deserialize(&symbol.to_vec());
                                        if let Some(decoded) = decoder.decode(packet) {
                                            // 1. DECRYPT
                                            let len_bytes: [u8;2] = decoded[0..2].try_into().unwrap();
                                            let real_len = u16::from_be_bytes(len_bytes) as usize;
                                            if decoded.len() >= 2 + real_len {
                                                let valid_payload = &decoded[2..2+real_len];
                                                let (nonce_bytes, ciphertext) = valid_payload.split_at(24);
                                                let nonce = XNonce::from_slice(nonce_bytes);
                                                
                                                if let Ok(ip_packet) = cipher_clone.decrypt(nonce, ciphertext) {
                                                    // 2. WRITE TO KERNEL (Internet Access!)
                                                    vpn_writer.write(&ip_packet).ok();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Err(_) => break,
                    }
                }
            },
            Err(e) => println!("Connection Error: {}", e),
        }
    }
}