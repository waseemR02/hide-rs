//! Decoding functionality for steganography

use crate::bltm::BLTM3x3;
use crate::error::HideError;
use crate::img::StegoImage;
use crate::utils;
use crate::Result;
use bitvec::prelude::*;
use std::path::Path;

/// Message format version expected by the decoder
const EXPECTED_FORMAT_VERSION: u8 = 1;

/// Header size in bytes
const HEADER_SIZE: usize = 8;

/// Decodes a message from a steganography image using BLTM method
pub struct Decoder {
    /// The BLTM used for decoding
    bltm: BLTM3x3,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    /// Create a new decoder with a 3x3 BLTM
    pub fn new() -> Self {
        Self {
            bltm: BLTM3x3::new(),
        }
    }

    /// Decode a single pixel to extract message bits
    ///
    /// # Arguments
    /// * `r` - Red component value (0-255)
    /// * `g` - Green component value (0-255)
    /// * `b` - Blue component value (0-255)
    ///
    /// # Returns
    /// * 3 bits of the hidden message
    pub fn decode_pixel(&self, r: u8, g: u8, b: u8) -> BitVec<u8, Msb0> {
        // Step 5-6: Extract LSBs to form stego vector vs
        let mut stego_vector = BitVec::<u8, Msb0>::new();
        stego_vector.push(r & 1 != 0); // LSB of R
        stego_vector.push(g & 1 != 0); // LSB of G
        stego_vector.push(b & 1 != 0); // LSB of B

        // Step 7: Find v^T = vs^T (transpose not needed for a single vector)

        // Step 8-9: m = (A × v^T)^T
        self.matrix_multiply(&stego_vector)
    }

    /// Matrix-vector multiplication: A × v
    fn matrix_multiply(&self, v: &BitSlice<u8, Msb0>) -> BitVec<u8, Msb0> {
        let columns = self.bltm.columns();
        let mut result = BitVec::<u8, Msb0>::new();

        // For each row of the matrix
        for i in 0..3 {
            // Calculate dot product of row i with vector v
            let mut bit = false;
            for j in 0..3 {
                // Extract the bit from A at position (i,j)
                // Note: columns are stored from right to left, so we access as columns[2-j]
                // For a lower triangular matrix, if i < j, the value is 0
                if i >= j {
                    let matrix_bit = columns[2 - j][i];
                    bit ^= matrix_bit && v[j]; // XOR with the AND of matrix bit and vector bit
                }
            }
            result.push(bit);
        }

        result
    }

    /// Extract the header from encoded data
    ///
    /// # Arguments
    /// * `bits` - The first HEADER_SIZE*8 bits from the stego image
    ///
    /// # Returns
    /// * The message format version and length
    fn extract_header(&self, bits: &BitVec<u8, Msb0>) -> Result<(u8, u32)> {
        if bits.len() < HEADER_SIZE * 8 {
            return Err(HideError::NoMessageFound);
        }

        // Convert header bits to bytes
        let header_bytes = utils::bits_to_bytes(&bits[..HEADER_SIZE * 8]);

        // Extract format version
        let format_version = header_bytes[0];

        // Verify format version
        if format_version != EXPECTED_FORMAT_VERSION {
            return Err(HideError::InvalidParameters(format!(
                "Unsupported message format version: {}",
                format_version
            )));
        }

        // Extract message length (big endian)
        let message_length = ((header_bytes[1] as u32) << 24)
            | ((header_bytes[2] as u32) << 16)
            | ((header_bytes[3] as u32) << 8)
            | (header_bytes[4] as u32);

        Ok((format_version, message_length))
    }

    /// Decode a message from an image
    ///
    /// # Arguments
    /// * `stego_image` - The image containing the hidden message
    ///
    /// # Returns
    /// * The extracted message bytes
    pub fn decode(&self, stego_image: &StegoImage) -> Result<Vec<u8>> {
        // Calculate the total number of bits we can extract
        let total_bits = (stego_image.width() * stego_image.height() * 3) as usize;

        // Check if the image is big enough to contain a header
        if total_bits < HEADER_SIZE * 8 {
            return Err(HideError::NoMessageFound);
        }

        // Extract all message bits from the image
        let mut all_bits = BitVec::<u8, Msb0>::with_capacity(total_bits);

        // Process each pixel to extract embedded bits
        let mut pixel_count = 0;
        let required_pixels = (HEADER_SIZE * 8 + 2) / 3; // Pixels needed for header plus a bit extra

        for y in 0..stego_image.height() {
            for x in 0..stego_image.width() {
                // Get the current pixel
                let pixel = stego_image.get_pixel_rgb(x, y)?;

                // Decode the pixel to extract message bits
                let pixel_bits = self.decode_pixel(pixel.0[0], pixel.0[1], pixel.0[2]);
                all_bits.extend_from_bitslice(&pixel_bits);

                pixel_count += 1;

                // After we've processed enough pixels for the header, extract and check it
                if pixel_count == required_pixels {
                    let (_, message_length) = self.extract_header(&all_bits)?;

                    // Calculate how many pixels we need in total
                    let message_bits = message_length as usize * 8;
                    let total_bits_needed = HEADER_SIZE * 8 + message_bits;
                    let total_pixels_needed = (total_bits_needed + 2) / 3; // Ceiling division

                    // Check if the message will fit in the image
                    if total_pixels_needed > (stego_image.width() * stego_image.height()) as usize {
                        return Err(HideError::NoMessageFound);
                    }
                }
            }
        }

        // Extract the header
        let (_, message_length) = self.extract_header(&all_bits)?;

        // Calculate the total number of bits in the message (including header)
        let message_bits = message_length as usize * 8;
        let total_bits_needed = HEADER_SIZE * 8 + message_bits;

        // Check if we extracted enough bits
        if all_bits.len() < total_bits_needed {
            return Err(HideError::NoMessageFound);
        }

        // Extract the message bits (after the header)
        let message_start = HEADER_SIZE * 8;
        let message_end = message_start + message_bits;
        let message_bits = &all_bits[message_start..message_end];

        // Convert bits back to bytes
        let message_bytes = utils::bits_to_bytes(message_bits);

        Ok(message_bytes)
    }

    /// Decode a message from an image file
    ///
    /// # Arguments
    /// * `stego_image_path` - Path to the image containing the hidden message
    ///
    /// # Returns
    /// * The extracted message bytes
    pub fn decode_file<P: AsRef<Path>>(&self, stego_image_path: P) -> Result<Vec<u8>> {
        // Load the stego image
        let stego_image = StegoImage::from_file(stego_image_path)?;

        // Decode the message
        self.decode(&stego_image)
    }

    /// Get a reference to the BLTM used by this decoder
    pub fn bltm(&self) -> &BLTM3x3 {
        &self.bltm
    }
}

/// Create a new decoder with default settings
pub fn create_decoder() -> Decoder {
    Decoder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::Encoder;
    use crate::img::create_rgb_image;
    use image::Rgb;

    #[test]
    fn test_decode_pixel_example() {
        let decoder = Decoder::new();

        // Input values from example
        let r = 123; // (01111011)2
        let g = 126; // (01111110)2
        let b = 135; // (10000111)2

        // Decode the pixel
        let message_bits = decoder.decode_pixel(r, g, b);

        // Expected message bits from example: (110)2
        let expected = bitvec![u8, Msb0; 1, 1, 0];
        assert_eq!(message_bits, expected, "Expected message bits [1, 1, 0]");
    }

    #[test]
    fn test_encode_decode_cycle() {
        let encoder = Encoder::new();
        let decoder = Decoder::new();

        // Create a test image
        let mut image = create_rgb_image(10, 10);

        // Fill with some test data
        for y in 0..10 {
            for x in 0..10 {
                let r = (x * 20) as u8;
                let g = (y * 20) as u8;
                let b = ((x + y) * 10) as u8;
                image.set_pixel_rgb(x, y, Rgb([r, g, b])).unwrap();
            }
        }

        // Original message to encode
        let original_message = b"Hello, world!";

        // Encode the message
        let stego_image = encoder.encode(image, original_message).unwrap();

        // Decode the message
        let decoded_message = decoder.decode(&stego_image).unwrap();

        // Verify the decoded message matches the original
        assert_eq!(decoded_message, original_message);
    }

    #[test]
    fn test_encode_decode_empty_message() {
        let encoder = Encoder::new();
        let decoder = Decoder::new();

        // Create a test image
        let image = create_rgb_image(10, 10);

        // Empty message to encode
        let original_message = b"";

        // Encode the message
        let stego_image = encoder.encode(image, original_message).unwrap();

        // Decode the message
        let decoded_message = decoder.decode(&stego_image).unwrap();

        // Verify the decoded message matches the original
        assert_eq!(decoded_message, original_message);
    }

    #[test]
    fn test_decode_without_message() {
        let decoder = Decoder::new();

        // Create an image without any hidden message
        let image = create_rgb_image(10, 10);

        // Attempt to decode - should fail
        let result = decoder.decode(&image);

        assert!(result.is_err());
        match result {
            Err(HideError::InvalidParameters(_)) => (), // Expected - the format version won't match
            Err(HideError::NoMessageFound) => (),       // This is also acceptable
            err => panic!("Unexpected result: {:?}", err),
        }
    }

    #[test]
    fn test_decode_invalid_format() {
        let decoder = Decoder::new();
        let mut image = create_rgb_image(10, 10);

        // Create an invalid header by directly manipulating LSBs
        // Set format version to 255 (invalid)
        image.set_lsb(0, 0, 0, true).unwrap();
        image.set_lsb(0, 0, 1, true).unwrap();
        image.set_lsb(0, 0, 2, true).unwrap();

        for y in 0..3 {
            for x in 0..3 {
                if !(y == 0 && x == 0) {
                    image.set_lsb(x, y, 0, true).unwrap();
                    image.set_lsb(x, y, 1, true).unwrap();
                    image.set_lsb(x, y, 2, true).unwrap();
                }
            }
        }

        // Attempt to decode - should fail with invalid format
        let result = decoder.decode(&image);

        assert!(result.is_err());
    }

    #[test]
    fn test_encode_decode_various_message_sizes() {
        let encoder = Encoder::new();
        let decoder = Decoder::new();

        // Create a test image
        let image = create_rgb_image(20, 20);

        // Test various message sizes
        let messages = [
            b"A".to_vec(),
            b"AB".to_vec(),
            b"Hello".to_vec(),
            b"This is a longer message to test".to_vec(),
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9], // Binary data
        ];

        for message in messages.iter() {
            // Encode the message
            let stego_image = encoder.encode(image.clone(), message).unwrap();

            // Decode the message
            let decoded_message = decoder.decode(&stego_image).unwrap();

            // Verify the decoded message matches the original
            assert_eq!(
                &decoded_message,
                message,
                "Failed for message of size {}",
                message.len()
            );
        }
    }

    #[test]
    fn test_encode_decode_edge_cases() {
        let encoder = Encoder::new();
        let decoder = Decoder::new();

        // Create a minimal sized image that can hold the header + 1 byte message
        // Header is 8 bytes = 64 bits, needs 22 pixels (each pixel stores 3 bits)
        // Plus 1 byte message = 8 bits, needs 3 more pixels
        // Total: 25 pixels, so 6x5 image is sufficient (30 pixels)
        let image = create_rgb_image(6, 5);

        // Message with special characters (small enough to fit)
        let message = b"\x00\x01\xFE";

        // Encode the message
        let stego_image = encoder.encode(image, message).unwrap();

        // Decode the message
        let decoded_message = decoder.decode(&stego_image).unwrap();

        // Verify the decoded message matches the original
        assert_eq!(decoded_message, message);
    }
}
