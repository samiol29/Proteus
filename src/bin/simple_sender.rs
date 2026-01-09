use std::net::UdpSocket;
use std::thread;
use std::time::Duration;
use dotenv::dotenv;
use std::env;

fn main() {
    println!("--- PROTEUS DEBUG SENDER ---");

    // 1. Load Secrets
    dotenv().ok();
    let target_ip = env::var("TARGET_IP").expect("Check .env file!");

    println!("[INFO] Target loaded from .env: [REDACTED]"); 
    // It will actually use the IP, just hiding it from print

    // 2. Bind to the OS
    // We bind to 0.0.0.0 to let the OS choose the best route
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");

    println!("[SENDING] Blasting packets... Check your phone.");

    let mut count = 0;
    loop {
        count += 1;
        let message = format!("HELLO FROM RUST #{}", count);

        // 3. Send pure text (Same as Netcat)
        match socket.send_to(message.as_bytes(), &target_ip) {
            Ok(_) => println!("Sent packet #{}", count),
            Err(e) => println!("FAILED to send: {}", e),
        }

        thread::sleep(Duration::from_secs(1));
    }
}