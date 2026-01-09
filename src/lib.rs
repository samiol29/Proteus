use serde::{Serialize, Deserialize};

// --- CONFIGURATION ---
pub const SERVER_ADDR: &str = "127.0.0.1:8080";

// UDP Packet Limit (MTU safe)
pub const MAX_UDP_SIZE: usize = 1400;

// RaptorQ Symbol Size (Must be identical on Client and Server)
// We choose 1024. This fits easily inside a 1400 byte UDP packet.
pub const SYMBOL_SIZE: u16 = 1024; 

// --- THE PROTOCOL ---
#[derive(Serialize, Deserialize, Debug)]
pub enum ProteusPacket {
    Data {
        seq: u32,
        payload: Vec<u8>,
    },
    Control {
        current_rank: u32,
        is_complete: bool,
    }
}