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
use proteus_core::{SYMBOL_SIZE}; // From our shared library

fn main() {
    println!("--- PROTEUS NODE [FINAL MERGER] ---");
    println!("I am a Shadow-TCP Stack sending RaptorQ Symbols.");
    
    // --- PART 1: PREPARE THE PAYLOAD (PHASE 1 LOGIC) ---
    println!("[1] preparing payload...");
    let plaintext = b"PROTEUS SECRET: The Shadow-TCP stack is fully operational. We are invisible.";
    
    // Encrypt
    let key_bytes = [0u8; 32]; // Zero key for demo simplicity
    let cipher = XChaCha20Poly1305::new(&key_bytes.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted = cipher.encrypt(&nonce, plaintext.as_ref()).unwrap();
    
    let mut blob = nonce.to_vec();
    blob.extend(encrypted);

    // Encode (RaptorQ)
    // We create the encoder ONCE and generate symbols on demand later
    let encoder = Encoder::with_defaults(&blob, SYMBOL_SIZE);
    let mut packet_counter = 0;

    println!("[SUCCESS] Payload Encrypted & Encoder Ready.");

    // --- PART 2: SETUP SHADOW STACK (PHASE 3 LOGIC) ---
    let mut device = TunTapInterface::new("tun0", Medium::Ethernet)
        .expect("Failed to create TUN. Run with SUDO.");

    let eth_addr = EthernetAddress([0x02, 0, 0, 0, 0, 0x01]);
    let config = Config::new(HardwareAddress::Ethernet(eth_addr));
    let mut iface = Interface::new(config, &mut device, Instant::now());

    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs.push(IpCidr::new(Ipv4Address::new(10, 0, 0, 2).into(), 24)).unwrap();
    });

    let mut sockets = SocketSet::new(vec![]);
    // Increase buffer size to handle RaptorQ symbols
    let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    
    let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    let tcp_handle = sockets.add(tcp_socket);

    println!("Listening on 10.0.0.2:80...");
    println!("Waiting for connection to blast symbols...");

    // --- PART 3: THE EVENT LOOP ---
    loop {
        let timestamp = Instant::now();
        iface.poll(timestamp, &mut device, &mut sockets);

        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
        
        // A. Maintain Listening State
        if !socket.is_open() {
            socket.listen(80).ok();
            packet_counter = 0; // Reset counter for new clients
        }

        // B. If Connected: BLAST DATA
        if socket.may_send() {
            // We implement "Rank-Based Flow Control" implicitly here.
            // In a real app, we'd wait for an ACK. Here, we stream slowly.
            
            // Get the next Repair Symbol from RaptorQ
            let packets = encoder.get_encoded_packets(1); // Get 1 new symbol
            let symbol = &packets[0]; // We know we asked for 1
            
            // Serialize
            let data = symbol.serialize();
            
            // Send if we have buffer space
            // We prefix with "PROT:" so the human eye can see it's our protocol
            if socket.send_slice(b"PROT:").is_ok() {
                match socket.send_slice(&data) {
                    Ok(_) => {
                        packet_counter += 1;
                         // Print a dot every 10 packets to avoid spamming logs
                        if packet_counter % 10 == 0 {
                            print!("."); 
                            use std::io::Write;
                            std::io::stdout().flush().unwrap();
                        }
                    },
                    Err(_) => { /* Buffer full, wait for next loop */ }
                }
            }
        }
    }
}