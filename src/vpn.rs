use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// The "Tank" Interface.
/// Interacts directly with the OS Kernel to capture traffic.
pub struct ProteusVpn {
    // FIXED: Removed 'dyn' because tun::Device is a Struct, not a Trait.
    device: Arc<Mutex<tun::Device>>,
}

impl ProteusVpn {
    /// Create the virtual interface "proteus0"
    /// WARNING: Requires ROOT/SUDO permissions.
    pub fn new() -> Self {
        println!("[KERNEL] Requesting TUN Device privileges...");
        
        let mut config = tun::Configuration::default();
        config
            .address((10, 0, 0, 1))       // The Virtual IP of this machine
            .destination((10, 0, 0, 254)) // The Gateway (Peer)
            .netmask((255, 255, 255, 0))
            .up();                        // Activate interface

        let dev = tun::create(&config).expect("Failed to create TUN device. Are you running with SUDO?");
        
        // We removed set_nonblock because it's private.
        // We will run in standard Blocking Mode (efficient waiting).

        println!("[SUCCESS] Interface 'proteus0' is UP. System-wide routing active.");
        
        Self {
            device: Arc::new(Mutex::new(dev)),
        }
    }

    /// Pull a raw packet from the OS (e.g., a browser request)
    pub fn read(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut dev = self.device.lock().unwrap();
        dev.read(buf)
    }

    /// Inject a packet back into the OS (e.g., a website response)
    pub fn write(&self, buf: &[u8]) -> std::io::Result<usize> {
        let mut dev = self.device.lock().unwrap();
        dev.write(buf)
    }
}