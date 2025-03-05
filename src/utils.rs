use crate::error::HideError;
use crate::Result;
use bitvec::prelude::*;

/// Extract `k` least significant bits from a byte
///
/// # Arguments
/// * `byte` - The byte to extract bits from
/// * `k` - Number of least significant bits to extract (1-8)
///
/// # Returns
/// * A u8 containing the k LSBs (right-aligned)
///
/// # Example
/// ```
/// use hide_rs::utils::get_lsbs;
///
/// let byte = 0b10101101;
/// let lsbs = get_lsbs(byte, 3).unwrap();
/// assert_eq!(lsbs, 0b00000101); // The 3 LSBs are 101
/// ```
pub fn get_lsbs(byte: u8, k: u8) -> Result<u8> {
    if k == 0 || k > 8 {
        return Err(HideError::InvalidParameters(format!(
            "Invalid number of bits: {}. Must be between 1 and 8",
            k
        )));
    }

    // Create a mask with k 1's in the least significant positions
    // Fix for "attempt to shift left with overflow" when k=8
    let mask = if k == 8 { 0xFF } else { (1u8 << k) - 1 };

    // Apply the mask to extract the k LSBs
    Ok(byte & mask)
}

/// Set `k` least significant bits in a byte
///
/// # Arguments
/// * `byte` - The byte to modify
/// * `bits` - The new bits to set (only k LSBs of this value are used)
/// * `k` - Number of least significant bits to replace (1-8)
///
/// # Returns
/// * The modified byte with k LSBs replaced
///
/// # Example
/// ```
/// use hide_rs::utils::set_lsbs;
///
/// let byte = 0b10101100;
/// let new_byte = set_lsbs(byte, 0b00000101, 3).unwrap();
/// assert_eq!(new_byte, 0b10101101); // The 3 LSBs are replaced with 101
/// ```
pub fn set_lsbs(byte: u8, bits: u8, k: u8) -> Result<u8> {
    if k == 0 || k > 8 {
        return Err(HideError::InvalidParameters(format!(
            "Invalid number of bits: {}. Must be between 1 and 8",
            k
        )));
    }

    // Create a mask with k 1's in the least significant positions
    // Fix for "attempt to shift left with overflow" when k=8
    let mask = if k == 8 { 0xFF } else { (1u8 << k) - 1 };

    // Clear the k LSBs in the original byte
    let cleared = byte & !mask;

    // Ensure that only k bits from the new value are used
    let new_bits = bits & mask;

    // Combine the cleared byte with the new bits
    Ok(cleared | new_bits)
}

/// Convert a sequence of bytes to a bit vector
///
/// # Arguments
/// * `bytes` - The byte slice to convert
///
/// # Returns
/// * A bit vector containing all bits from the input bytes
pub fn bytes_to_bits(bytes: &[u8]) -> BitVec<u8, Msb0> {
    let mut bits = BitVec::<u8, Msb0>::with_capacity(bytes.len() * 8);
    for &byte in bytes {
        for i in 0..8 {
            bits.push(((byte >> (7 - i)) & 1) == 1);
        }
    }
    bits
}

/// Convert a bit vector to a sequence of bytes
///
/// # Arguments
/// * `bits` - The bits to convert
///
/// # Returns
/// * A vector of bytes constructed from the input bits
/// * If the bit vector length is not a multiple of 8, the last byte is padded with 0s
pub fn bits_to_bytes(bits: &BitSlice<u8, Msb0>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((bits.len() + 7) / 8);
    let mut byte = 0u8;
    let mut bit_count = 0;

    for bit in bits.iter() {
        byte = (byte << 1) | (*bit as u8);
        bit_count += 1;

        if bit_count == 8 {
            bytes.push(byte);
            byte = 0;
            bit_count = 0;
        }
    }

    // Add the last byte if there are remaining bits
    if bit_count > 0 {
        // Shift the remaining bits to align with the most significant positions
        byte <<= 8 - bit_count;
        bytes.push(byte);
    }

    bytes
}

/// Split a bit vector into chunks of a specified size
///
/// # Arguments
/// * `bits` - The bit vector to split
/// * `chunk_size` - Size of each chunk in bits
///
/// # Returns
/// * A vector of bit vectors, each containing a chunk of the original
/// * The last chunk may be padded with 0s if necessary
pub fn split_bits(bits: &BitVec<u8, Msb0>, chunk_size: usize) -> Result<Vec<BitVec<u8, Msb0>>> {
    if chunk_size == 0 {
        return Err(HideError::InvalidParameters(
            "Chunk size cannot be zero".to_string(),
        ));
    }

    let mut chunks = Vec::new();
    let mut i = 0;

    while i < bits.len() {
        let end = std::cmp::min(i + chunk_size, bits.len());
        let mut chunk = bits[i..end].to_bitvec();

        // Pad the last chunk if necessary
        if chunk.len() < chunk_size {
            chunk.resize(chunk_size, false);
        }

        chunks.push(chunk);
        i += chunk_size;
    }

    Ok(chunks)
}

/// Join multiple bit vectors into a single bit vector
///
/// # Arguments
/// * `chunks` - Vector of bit vectors to join
/// * `total_bits` - Optional total number of bits to keep (truncates the result)
///
/// # Returns
/// * A single bit vector containing all the input bits
pub fn join_bits(chunks: &[BitVec<u8, Msb0>], total_bits: Option<usize>) -> BitVec<u8, Msb0> {
    let mut result = BitVec::<u8, Msb0>::new();

    for chunk in chunks {
        result.extend_from_bitslice(chunk);
    }

    // Truncate to specified length if needed
    if let Some(len) = total_bits {
        if result.len() > len {
            result.truncate(len);
        }
    }

    result
}

/// Get a single bit from a byte (0 to 7, where 0 is the most significant bit)
///
/// # Arguments
/// * `byte` - The byte to extract a bit from
/// * `bit_position` - Position of the bit to extract (0-7)
///
/// # Returns
/// * Boolean value of the specified bit
pub fn get_bit(byte: u8, bit_position: u8) -> Result<bool> {
    if bit_position > 7 {
        return Err(HideError::InvalidParameters(format!(
            "Invalid bit position: {}. Must be between 0 and 7",
            bit_position
        )));
    }

    Ok(((byte >> (7 - bit_position)) & 1) == 1)
}

/// Set a single bit in a byte (0 to 7, where 0 is the most significant bit)
///
/// # Arguments
/// * `byte` - The byte to modify
/// * `bit_position` - Position of the bit to set (0-7)
/// * `value` - New value for the bit (true = 1, false = 0)
///
/// # Returns
/// * The modified byte
pub fn set_bit(byte: u8, bit_position: u8, value: bool) -> Result<u8> {
    if bit_position > 7 {
        return Err(HideError::InvalidParameters(format!(
            "Invalid bit position: {}. Must be between 0 and 7",
            bit_position
        )));
    }

    let pos = 7 - bit_position;

    if value {
        Ok(byte | (1 << pos))
    } else {
        Ok(byte & !(1 << pos))
    }
}

/// Get the least significant bit of a byte
pub fn get_lsb(byte: u8) -> bool {
    (byte & 1) == 1
}

/// Set the least significant bit of a byte
pub fn set_lsb(byte: &mut u8, bit: bool) {
    if bit {
        *byte |= 1;
    } else {
        *byte &= !1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_lsbs() {
        // Test extracting different numbers of LSBs
        assert_eq!(get_lsbs(0b10101101, 1).unwrap(), 0b00000001);
        assert_eq!(get_lsbs(0b10101101, 3).unwrap(), 0b00000101);
        assert_eq!(get_lsbs(0b10101101, 8).unwrap(), 0b10101101);

        // Test with different byte values
        assert_eq!(get_lsbs(0b00000000, 4).unwrap(), 0b00000000);
        assert_eq!(get_lsbs(0b11111111, 4).unwrap(), 0b00001111);
        assert_eq!(get_lsbs(0b10000001, 2).unwrap(), 0b00000001);

        // Test error cases
        assert!(get_lsbs(0b10101101, 0).is_err());
        assert!(get_lsbs(0b10101101, 9).is_err());
    }

    #[test]
    fn test_set_lsbs() {
        // Test setting different numbers of LSBs
        assert_eq!(set_lsbs(0b10101100, 0b00000001, 1).unwrap(), 0b10101101);
        assert_eq!(set_lsbs(0b10101100, 0b00000101, 3).unwrap(), 0b10101101);
        assert_eq!(set_lsbs(0b00000000, 0b11111111, 8).unwrap(), 0b11111111);

        // Test with different byte values
        assert_eq!(set_lsbs(0b11111111, 0b00000000, 4).unwrap(), 0b11110000);
        assert_eq!(set_lsbs(0b00000000, 0b11111111, 4).unwrap(), 0b00001111);

        // Test that only k LSBs are used from the input value
        assert_eq!(set_lsbs(0b10101100, 0b11111111, 3).unwrap(), 0b10101111);

        // Test error cases
        assert!(set_lsbs(0b10101100, 0b00000001, 0).is_err());
        assert!(set_lsbs(0b10101100, 0b00000001, 9).is_err());
    }

    #[test]
    fn test_bytes_to_bits_and_back() {
        // Test conversion of various byte patterns
        let test_cases = vec![
            vec![0xAA, 0x55, 0xF0],
            vec![0x00, 0xFF, 0x0F],
            vec![0x12, 0x34, 0x56, 0x78],
            vec![0xFF],
            vec![0x00],
        ];

        for bytes in test_cases {
            let bits = bytes_to_bits(&bytes);
            let result = bits_to_bytes(&bits);
            assert_eq!(bytes, result);
        }
    }

    #[test]
    fn test_bytes_to_bits_partial() {
        // Create a bitvec with a length that's not a multiple of 8
        let bytes = vec![0xAA, 0x55];
        let mut bits = bytes_to_bits(&bytes);

        // Remove 3 bits, so we have 13 bits (not a multiple of 8)
        bits.truncate(13);

        // Convert back to bytes
        let result = bits_to_bytes(&bits);

        // Should be 2 bytes, with the second one having 5 bits of data
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0xAA);

        // 5 bits from 0x55 - this should be 0x50 (10101000)
        assert_eq!(result[1], 0x50);
    }

    #[test]
    fn test_split_and_join_bits() {
        // Create a test bit vector
        let data = vec![0xA5, 0xF0, 0x3C]; // 10100101 11110000 00111100
        let bits = bytes_to_bits(&data);

        // Split into chunks of 4 bits
        let chunks = split_bits(&bits, 4).unwrap();

        // Should get 6 chunks of 4 bits
        assert_eq!(chunks.len(), 6);

        // Join the chunks back
        let joined = join_bits(&chunks, Some(bits.len()));

        // Should be the same as the original
        assert_eq!(joined, bits);

        // Manually check the first chunk
        assert!(chunks[0][0]); // 1
        assert!(!chunks[0][1]); // 0
        assert!(chunks[0][2]); // 1
        assert!(!chunks[0][3]); // 0

        // Manually check the second chunk
        assert!(!chunks[1][0]); // 0
        assert!(chunks[1][1]); // 1
        assert!(!chunks[1][2]); // 0
        assert!(chunks[1][3]); // 1
    }

    #[test]
    fn test_split_bits_with_padding() {
        // Create a bit vector with a length that's not divisible by the chunk size
        let data = vec![0xA5]; // 10100101 (8 bits)
        let bits = bytes_to_bits(&data);

        // Split into chunks of 3 bits (should get 3 chunks: 2 full and 1 padded)
        let chunks = split_bits(&bits, 3).unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 3); // 101 (first 3 bits)
        assert_eq!(chunks[1].len(), 3); // 001 (next 3 bits)
        assert_eq!(chunks[2].len(), 3); // 01_ (last 2 bits + 1 padding bit)

        assert!(chunks[0][0]); // 1
        assert!(!chunks[0][1]); // 0
        assert!(chunks[0][2]); // 1

        assert!(!chunks[1][0]); // 0
        assert!(!chunks[1][1]); // 0
        assert!(chunks[1][2]); // 1

        assert!(!chunks[2][0]); // 0
        assert!(chunks[2][1]); // 1
        assert!(!chunks[2][2]); // 0 (padding)
    }

    #[test]
    fn test_get_set_bit() {
        let byte = 0b10101010;

        // Test getting bits
        assert!(get_bit(byte, 0).unwrap());
        assert!(!get_bit(byte, 1).unwrap());
        assert!(!get_bit(byte, 7).unwrap());

        // Test setting bits
        assert_eq!(set_bit(byte, 1, true).unwrap(), 0b11101010);
        assert_eq!(set_bit(byte, 0, false).unwrap(), 0b00101010);
        assert_eq!(set_bit(byte, 7, true).unwrap(), 0b10101011);

        // Test error cases
        assert!(get_bit(byte, 8).is_err());
        assert!(set_bit(byte, 8, true).is_err());
    }

    #[test]
    fn test_lsb_operations() {
        // Test get_lsb
        assert!(!get_lsb(0b10101010));
        assert!(get_lsb(0b10101011));

        // Test set_lsb
        let mut byte = 0b10101010;
        set_lsb(&mut byte, true);
        assert_eq!(byte, 0b10101011);

        set_lsb(&mut byte, false);
        assert_eq!(byte, 0b10101010);
    }
}
