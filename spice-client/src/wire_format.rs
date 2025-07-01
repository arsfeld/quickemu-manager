use crate::error::{Result, SpiceError};
use crate::protocol::SpiceDataHeader;
use crate::transport::Transport;
use binrw::{BinRead, BinWrite};
use std::io::Cursor;

/// Trait for SPICE protocol structures that can be serialized to/from wire format
///
/// This trait ensures that we have explicit control over the byte layout
/// of protocol messages, avoiding issues with Rust's automatic struct padding.
pub trait SpiceWireFormat: Sized {
    /// Returns the exact size of this structure on the wire
    fn wire_size() -> usize;

    /// Writes this structure to a byte buffer in SPICE wire format
    fn write_to_wire(&self, buf: &mut Vec<u8>) -> Result<()>;

    /// Reads this structure from a byte buffer in SPICE wire format
    fn read_from_wire(buf: &[u8]) -> Result<Self>;
}

/// Helper trait for writing primitive types to buffers
pub trait WriteWire {
    fn write_u8(&mut self, val: u8);
    fn write_u16_le(&mut self, val: u16);
    fn write_u32_le(&mut self, val: u32);
    fn write_u64_le(&mut self, val: u64);
    fn write_bytes(&mut self, bytes: &[u8]);
}

impl WriteWire for Vec<u8> {
    fn write_u8(&mut self, val: u8) {
        self.push(val);
    }

    fn write_u16_le(&mut self, val: u16) {
        self.extend_from_slice(&val.to_le_bytes());
    }

    fn write_u32_le(&mut self, val: u32) {
        self.extend_from_slice(&val.to_le_bytes());
    }

    fn write_u64_le(&mut self, val: u64) {
        self.extend_from_slice(&val.to_le_bytes());
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}

/// Helper functions for reading from byte slices
pub trait ReadWire {
    fn read_u8(&self, offset: usize) -> Result<u8>;
    fn read_u16_le(&self, offset: usize) -> Result<u16>;
    fn read_u32_le(&self, offset: usize) -> Result<u32>;
    fn read_u64_le(&self, offset: usize) -> Result<u64>;
    fn read_bytes(&self, offset: usize, len: usize) -> Result<&[u8]>;
}

impl ReadWire for [u8] {
    fn read_u8(&self, offset: usize) -> Result<u8> {
        if offset >= self.len() {
            return Err(crate::error::SpiceError::Protocol(
                "Buffer too small for u8".into(),
            ));
        }
        Ok(self[offset])
    }

    fn read_u16_le(&self, offset: usize) -> Result<u16> {
        if offset + 2 > self.len() {
            return Err(crate::error::SpiceError::Protocol(
                "Buffer too small for u16".into(),
            ));
        }
        Ok(u16::from_le_bytes([self[offset], self[offset + 1]]))
    }

    fn read_u32_le(&self, offset: usize) -> Result<u32> {
        if offset + 4 > self.len() {
            return Err(crate::error::SpiceError::Protocol(
                "Buffer too small for u32".into(),
            ));
        }
        Ok(u32::from_le_bytes([
            self[offset],
            self[offset + 1],
            self[offset + 2],
            self[offset + 3],
        ]))
    }

    fn read_u64_le(&self, offset: usize) -> Result<u64> {
        if offset + 8 > self.len() {
            return Err(crate::error::SpiceError::Protocol(
                "Buffer too small for u64".into(),
            ));
        }
        Ok(u64::from_le_bytes([
            self[offset],
            self[offset + 1],
            self[offset + 2],
            self[offset + 3],
            self[offset + 4],
            self[offset + 5],
            self[offset + 6],
            self[offset + 7],
        ]))
    }

    fn read_bytes(&self, offset: usize, len: usize) -> Result<&[u8]> {
        if offset + len > self.len() {
            return Err(crate::error::SpiceError::Protocol(
                "Buffer too small for bytes".into(),
            ));
        }
        Ok(&self[offset..offset + len])
    }
}

/// Read a complete message from the transport
pub async fn read_message(
    transport: &mut Box<dyn Transport>,
) -> Result<(SpiceDataHeader, Vec<u8>)> {
    // Read header (18 bytes)
    let mut header_buf = vec![0u8; 18];
    let mut total_read = 0;

    while total_read < 18 {
        let n = transport
            .read(&mut header_buf[total_read..])
            .await
            .map_err(|e| SpiceError::Io(e))?;
        if n == 0 {
            return Err(SpiceError::Protocol(
                "Connection closed while reading header".to_string(),
            ));
        }
        total_read += n;
    }

    // Parse header using binrw
    let mut cursor = Cursor::new(&header_buf);
    let header = SpiceDataHeader::read(&mut cursor)
        .map_err(|e| SpiceError::Protocol(format!("Failed to parse header: {}", e)))?;

    // Read message data
    let mut data = vec![0u8; header.msg_size as usize];
    total_read = 0;

    while total_read < header.msg_size as usize {
        let n = transport
            .read(&mut data[total_read..])
            .await
            .map_err(|e| SpiceError::Io(e))?;
        if n == 0 {
            return Err(SpiceError::Protocol(
                "Connection closed while reading message data".to_string(),
            ));
        }
        total_read += n;
    }

    Ok((header, data))
}

/// Write a complete message to the transport
pub async fn write_message(
    transport: &mut Box<dyn Transport>,
    header: &SpiceDataHeader,
    data: &[u8],
) -> Result<()> {
    // Serialize header using binrw
    let mut header_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut header_bytes);
    header
        .write(&mut cursor)
        .map_err(|e| SpiceError::Protocol(format!("Failed to serialize header: {}", e)))?;

    // Write header
    transport
        .write_all(&header_bytes)
        .await
        .map_err(|e| SpiceError::Io(e))?;

    // Write data
    transport
        .write_all(data)
        .await
        .map_err(|e| SpiceError::Io(e))?;

    // Flush to ensure data is sent
    transport.flush().await.map_err(|e| SpiceError::Io(e))?;

    Ok(())
}
