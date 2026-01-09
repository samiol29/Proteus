use std::fmt::Write;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{TunTapInterface, Medium};
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, HardwareAddress};
use raptorq::Encoder;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce
};
use proteus_core::SYMBOL_SIZE;

fn main() {
    println!("--- PROTEUS NODE v2 (DYNAMIC HANDSHAKE) ---");
    
    // --- PART 1: PREPARE PAYLOAD ---
    // You can change this text to ANYTHING now. Long or short.
    let plaintext = b"PROTEUS UPDATE: We successfully negotiated the packet size. The protocol is now dynamic.";
    println!("[1] Payload size: {} bytes", plaintext.len());
    
    // Encrypt
    let key_bytes = [0u8; 32]; 
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted = cipher.encrypt(&nonce, plaintext.as_ref()).unwrap();
    
    let mut blob = nonce.to_vec();
    blob.extend(encrypted);

    // CRITICAL: Calculate the exact size to send during handshake
    let total_blob_size = blob.len() as u64;
    println!("[INFO] Total Encrypted Size: {} bytes", total_blob_size);

    let encoder = Encoder::with_defaults(&blob, SYMBOL_SIZE);

    // --- PART 2: SETUP SHADOW STACK ---
    let mut device = TunTapInterface::new("tun0", Medium::Ethernet)
        .expect("Failed to create TUN. Run with SUDO.");

    let eth_addr = EthernetAddress([0x02, 0, 0, 0, 0, 0x01]);
    let config = Config::new(HardwareAddress::Ethernet(eth_addr));
    let mut iface = Interface::new(config, &mut device, Instant::now());

    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs.push(IpCidr::new(Ipv4Address::new(10, 0, 0, 2).into(), 24)).unwrap();
    });

    let mut sockets = SocketSet::new(vec![]);
    let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    let tcp_handle = sockets.add(tcp_socket);

    println!("Listening on 10.0.0.2:80...");

    // State tracking
    let mut handshake_sent = false;
    let mut packet_counter = 0;

    // --- PART 3: EVENT LOOP ---
    loop {
        let timestamp = Instant::now();
        iface.poll(timestamp, &mut device, &mut sockets);
        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
        
        // A. Reset state on new connection / disconnect
        if !socket.is_open() {
            socket.listen(80).ok();
            handshake_sent = false; // Reset for next client
            packet_counter = 0;
        }

        // B. Sending Logic
        if socket.may_send() {
            // STEP 1: Send the Handshake (Size)
            if !handshake_sent {
                // Convert u64 size to 8 bytes (Big Endian)
                let size_bytes = total_blob_size.to_be_bytes();
                
                match socket.send_slice(&size_bytes) {
                    Ok(_) => {
                        println!("\n[HANDSHAKE] Sent file size: {}", total_blob_size);
                        handshake_sent = true;
                    },
                    Err(_) => { /* Wait for buffer space */ }
                }
            } 
            // STEP 2: Stream Symbols (Only after handshake)
            else {
                let packets = encoder.get_encoded_packets(1);
                let symbol = &packets[0];
                let data = symbol.serialize();
                
                if socket.send_slice(b"PROT:").is_ok() {
                    match socket.send_slice(&data) {
                        Ok(_) => {
                            packet_counter += 1;
                            if packet_counter % 10 == 0 {
                                print!("."); 
                                use std::io::Write;
                                std::io::stdout().flush().unwrap();
                            }
                        },
                        Err(_) => { /* Buffer full */ }
                    }
                }
            }
        }
    }
}