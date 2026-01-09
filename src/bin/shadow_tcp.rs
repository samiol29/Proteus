use std::fmt::Write; // Needed for writing to the socket
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{TunTapInterface, Medium};
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, HardwareAddress}; // Added HardwareAddress

fn main() {
    println!("--- PROTEUS SHADOW-TCP SERVER ---");
    println!("I am a User-Space TCP Stack. I don't use the Kernel!");
    
    // 1. Create the TUN Interface
    let mut device = TunTapInterface::new("tun0", Medium::Ethernet)
        .expect("Failed to create TUN. Did you run with SUDO?");

    println!("[SUCCESS] Created TUN device 'tun0'");

    // 2. Configure the Network Stack
    // FIX: We wrap the address in HardwareAddress::Ethernet()
    let eth_addr = EthernetAddress([0x02, 0, 0, 0, 0, 0x01]);
    let config = Config::new(HardwareAddress::Ethernet(eth_addr));
    
    let mut iface = Interface::new(config, &mut device, Instant::now());

    // We accept traffic for 10.0.0.2/24
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs.push(IpCidr::new(Ipv4Address::new(10, 0, 0, 2).into(), 24)).unwrap();
    });

    // 3. Create a TCP Socket
    let mut sockets = SocketSet::new(vec![]);
    let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 1024]);
    let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 1024]);
    
    let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    let tcp_handle = sockets.add(tcp_socket);

    println!("Listening for TCP connections on 10.0.0.2:80...");

    // 4. The Event Loop
    loop {
        let timestamp = Instant::now();
        
        // Pump the interface
        iface.poll(timestamp, &mut device, &mut sockets);

        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
        
        // A. Listen if closed
        if !socket.is_open() {
            socket.listen(80).ok();
        }

        // B. Read/Write Data
        if socket.can_recv() {
            let mut data = [0u8; 1024];
            if let Ok(size) = socket.recv_slice(&mut data) {
                 let msg = String::from_utf8_lossy(&data[..size]);
                 println!("> [SHADOW-TCP] Received Encrypted Frame: {:?}", msg);
                 
                 // Reply
                 write!(socket, "Proteus Shadow-ACK\n").ok();
            }
        }
    }
}