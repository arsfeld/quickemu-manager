use binrw::BinRead;
use std::io::Cursor;
use spice_client::protocol::*;

fn main() {
    // Raw DrawCopy data from the logs (full 165 bytes)
    let raw_data = vec![
        0x00, 0x00, 0x00, 0x00, 0x8d, 0x00, 0x00, 0x00, // bbox: left=0, top=141
        0x00, 0x00, 0x00, 0x00, 0x8f, 0x00, 0x00, 0x00, // bbox: right=0, bottom=143
        0x09, 0x00, 0x00, 0x00, 0x00, 0x39, 0x00, 0x00, // clip_type + padding + data start
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // clip data continued
        0x00, 0x02, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, // Start of DrawCopyData
        0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x73, 0x05, 0x00, 0x00,
        0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x02, 0x00,
        0x00, 0x00, 0x08, 0x04, 0x09, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0xa8, 0xa8, 0xa8, 0x00,
        0xa8, 0xa8, 0xa8, 0x00, 0xa8, 0xa8, 0xa8, 0x00,
    ];
    
    // Let me also manually parse the first few fields to understand the layout
    println!("Manual parsing:");
    println!("bbox.left: {}", u32::from_le_bytes([raw_data[0], raw_data[1], raw_data[2], raw_data[3]]));
    println!("bbox.top: {}", u32::from_le_bytes([raw_data[4], raw_data[5], raw_data[6], raw_data[7]]));
    println!("bbox.right: {}", u32::from_le_bytes([raw_data[8], raw_data[9], raw_data[10], raw_data[11]]));
    println!("bbox.bottom: {}", u32::from_le_bytes([raw_data[12], raw_data[13], raw_data[14], raw_data[15]]));
    println!("Next 4 bytes (clip?): 0x{:02x} 0x{:02x} 0x{:02x} 0x{:02x}", 
        raw_data[16], raw_data[17], raw_data[18], raw_data[19]);

    println!("Raw data length: {} bytes", raw_data.len());
    println!("Raw data (hex):");
    for (i, chunk) in raw_data.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        println!();
    }

    // Try to parse as SpiceDrawBase
    let mut cursor = Cursor::new(&raw_data);
    match SpiceDrawBase::read(&mut cursor) {
        Ok(base) => {
            println!("\nParsed SpiceDrawBase:");
            println!("  bbox: {:?}", base.box_);
            println!("  clip: type={} (0x{:02x})", base.clip.clip_type, base.clip.clip_type);
            
            // The clip_type of 9 is invalid according to ClipType enum (only 0=None, 1=Rects)
            if base.clip.clip_type > 1 {
                println!("  WARNING: Invalid clip type! Expected 0 or 1, got {}", base.clip.clip_type);
            }
            
            // Continue parsing SpiceDrawCopyData
            let pos = cursor.position() as usize;
            println!("\nCursor position after base: {}", pos);
            
            if pos + 8 <= raw_data.len() {
                // Read src_image address manually
                let addr_bytes = &raw_data[pos..pos+8];
                let src_image = u64::from_le_bytes([
                    addr_bytes[0], addr_bytes[1], addr_bytes[2], addr_bytes[3],
                    addr_bytes[4], addr_bytes[5], addr_bytes[6], addr_bytes[7]
                ]);
                println!("src_image address: 0x{:x} ({})", src_image, src_image);
                
                // Analyze the address structure
                println!("Address breakdown:");
                println!("  Upper 32 bits: 0x{:x} ({})", src_image >> 32, src_image >> 32);
                println!("  Lower 32 bits: 0x{:x} ({})", src_image & 0xFFFFFFFF, src_image & 0xFFFFFFFF);
                
                // Check for special patterns
                if src_image > 0x1_0000_0000 {
                    println!("  WARNING: Address > 4GB, likely special encoding or cache reference");
                }
                
                // Try to decode if this looks like a cache reference
                if (src_image >> 32) > 0 {
                    let high = (src_image >> 32) as u32;
                    let low = (src_image & 0xFFFFFFFF) as u32;
                    println!("  Possible cache ref: surface_id=0x{:x}, offset=0x{:x}", high, low);
                }
                
                // Check what comes after the address (should be src_area rect)
                if pos + 24 <= raw_data.len() {
                    let src_area_left = u32::from_le_bytes([
                        raw_data[pos+8], raw_data[pos+9], raw_data[pos+10], raw_data[pos+11]
                    ]);
                    let src_area_top = u32::from_le_bytes([
                        raw_data[pos+12], raw_data[pos+13], raw_data[pos+14], raw_data[pos+15]
                    ]);
                    let src_area_right = u32::from_le_bytes([
                        raw_data[pos+16], raw_data[pos+17], raw_data[pos+18], raw_data[pos+19]
                    ]);
                    let src_area_bottom = u32::from_le_bytes([
                        raw_data[pos+20], raw_data[pos+21], raw_data[pos+22], raw_data[pos+23]
                    ]);
                    println!("src_area: ({},{}) to ({},{})", src_area_left, src_area_top, src_area_right, src_area_bottom);
                }
            }
        }
        Err(e) => {
            println!("Failed to parse SpiceDrawBase: {:?}", e);
        }
    }
}