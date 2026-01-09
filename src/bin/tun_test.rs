use std::io::Read;
// FIX: We must import 'AbstractDevice' to access .tun_name() and .read()
use tun::AbstractDevice; 

fn main() {
    println!("--- PROTEUS TUN INTERFACE TEST (FINAL) ---");
    println!("Attempting to open a virtual network card (requires SUDO)...");

    // 1. Configuration
    let mut config = tun::Configuration::default();
    
    config
        .address((10, 0, 0, 1))       
        .netmask((255, 255, 255, 0))  
        .up();                        

    // 2. Create Interface
    match tun::create(&config) {
        Ok(mut dev) => {
            // Now .tun_name() will work because we imported AbstractDevice
            let name = dev.tun_name().unwrap_or("tun0".to_string());
            println!("[SUCCESS] Created interface: {}", name);
            println!("Proteus is now listening for OS traffic on 10.0.0.1");
            println!("(Press Ctrl+C to stop)");

            // 3. Simple Loop
            let mut buf = [0u8; 1500];
            loop {
                match dev.read(&mut buf) {
                    Ok(amount) => {
                        println!("> Captured a packet from Kernel! Size: {} bytes", amount);
                    },
                    Err(e) => {
                        eprintln!("Error reading: {}", e);
                        break;
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("\n[FAILURE] Could not create TUN device.");
            eprintln!("Error: {}", e);
            eprintln!("HINT: Did you run with 'sudo'? Virtual cards require Root privileges.");
        }
    }
}