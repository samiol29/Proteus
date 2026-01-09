use std::net::UdpSocket;
use std::net::TcpStream;
use std::io::Write;
use std::sync::{Mutex, Arc}; // Added Arc

pub enum TransportType {
    Udp(UdpSocket),
    Tcp(Arc<Mutex<TcpStream>>), // FIXED: Wrapped in Arc for sharing
}

impl TransportType {
    pub fn send(&self, data: &[u8], target: &str) -> std::io::Result<()> {
        match self {
            TransportType::Udp(socket) => {
                socket.send_to(data, target).map(|_| ())
            }
            TransportType::Tcp(stream_lock) => {
                let mut stream = stream_lock.lock().unwrap();
                stream.write_all(data)?;
                Ok(())
            }
        }
    }
}