use std::time::{SystemTime, UNIX_EPOCH};
use std::convert::TryInto;

pub const HEADER_SIZE: usize = 12; // 4 bytes (Seq) + 8 bytes (Time)

#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    pub seq_id: u32,
    pub timestamp: u64, // Microseconds since UNIX EPOCH
}

impl PacketHeader {
    /// Create a new header for the current moment
    pub fn new(seq_id: u32) -> Self {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        
        Self {
            seq_id,
            timestamp: since_the_epoch.as_micros() as u64,
        }
    }

    /// Serialize struct into [u8; 12]
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        
        // Write Seq ID (First 4 bytes)
        bytes[0..4].copy_from_slice(&self.seq_id.to_be_bytes());
        
        // Write Timestamp (Next 8 bytes)
        bytes[4..12].copy_from_slice(&self.timestamp.to_be_bytes());
        
        bytes
    }

    /// Deserialize bytes back into struct
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEADER_SIZE {
            return None;
        }

        let seq_id = u32::from_be_bytes(bytes[0..4].try_into().ok()?);
        let timestamp = u64::from_be_bytes(bytes[4..12].try_into().ok()?);

        Some(Self { seq_id, timestamp })
    }
}

// ... keep existing PacketHeader code ...

pub const ACK_SIZE: usize = 12; // 4 bytes (Seq) + 8 bytes (Time)

#[derive(Debug)]
pub struct AckPacket {
    pub seq_id: u32,
    pub timestamp: u64,
}

impl AckPacket {
    pub fn new(seq_id: u32, timestamp: u64) -> Self {
        Self { seq_id, timestamp }
    }

    pub fn to_bytes(&self) -> [u8; ACK_SIZE] {
        let mut bytes = [0u8; ACK_SIZE];
        bytes[0..4].copy_from_slice(&self.seq_id.to_be_bytes());
        bytes[4..12].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < ACK_SIZE { return None; }
        let seq_id = u32::from_be_bytes(bytes[0..4].try_into().ok()?);
        let timestamp = u64::from_be_bytes(bytes[4..12].try_into().ok()?);
        Some(Self { seq_id, timestamp })
    }
}