//! Utility for raw data extraction from steganographic images without header validation

use crate::decoder::Decoder;
use crate::img::StegoImage;
use crate::Result;
use bitvec::prelude::*;

/// Extract raw data from a steganographic image without header validation
pub fn extract_raw_data(stego_image: &StegoImage) -> Result<Vec<u8>> {
    let decoder = Decoder::new();
    
    // Extract all bits from the image
    let mut all_bits = BitVec::<u8, Msb0>::new();
    
    // Process each pixel to extract embedded bits
    for y in 0..stego_image.height() {
        for x in 0..stego_image.width() {
            // Get the current pixel
            let pixel = stego_image.get_pixel_rgb(x, y)?;
            
            // Decode the pixel to extract message bits
            let pixel_bits = decoder.decode_pixel(pixel.0[0], pixel.0[1], pixel.0[2]);
            all_bits.extend_from_bitslice(&pixel_bits);
        }
    }
    
    // Convert all bits to bytes
    let bytes = bits_to_bytes(&all_bits);
    
    Ok(bytes)
}

/// Convert bits to bytes without any validation
fn bits_to_bytes(bits: &BitSlice<u8, Msb0>) -> Vec<u8> {
    let byte_count = (bits.len() + 7) / 8; // Ceiling division
    let mut bytes = vec![0u8; byte_count];
    
    for (i, byte) in bytes.iter_mut().enumerate() {
        let start_bit = i * 8;
        let end_bit = std::cmp::min(start_bit + 8, bits.len());
        
        // Fill byte with available bits
        for j in start_bit..end_bit {
            if bits[j] {
                *byte |= 1 << (7 - (j % 8));
            }
        }
    }
    
    bytes
}

/// Format the first N bytes of data in a human-readable way (hex and binary)
pub fn format_data_preview(data: &[u8], n: usize) -> String {
    let n = std::cmp::min(n, data.len());
    let mut result = String::new();
    
    result.push_str("Raw data preview:\n");
    
    // Header format info
    if n >= 8 {
        result.push_str(&format!("Potential header: \n"));
        result.push_str(&format!("  Format version: {} (expected: 1)\n", data[0]));
        
        // Extract message length (big endian)
        let message_length = ((data[1] as u32) << 24)
            | ((data[2] as u32) << 16)
            | ((data[3] as u32) << 8)
            | (data[4] as u32);
        result.push_str(&format!("  Message length: {} bytes\n", message_length));
        
        // Reserved bytes
        result.push_str(&format!("  Reserved bytes: {:02X} {:02X} {:02X}\n", data[5], data[6], data[7]));
    }
    
    // Hex view
    result.push_str("Hex view:\n");
    for i in 0..n {
        if i % 16 == 0 {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(&format!("{:04X}: ", i));
        }
        result.push_str(&format!("{:02X} ", data[i]));
    }
    
    // Binary view
    result.push_str("\n\nBinary view:\n");
    for i in 0..n {
        if i % 4 == 0 {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(&format!("{:04X}: ", i));
        }
        result.push_str(&format!("{:08b} ", data[i]));
    }
    
    result
}
