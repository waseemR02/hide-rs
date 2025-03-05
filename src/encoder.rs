//! Encoding functionality for steganography

use crate::bltm::BLTM3x3;
use crate::error::HideError;
use crate::img::StegoImage;
use crate::utils;
use crate::Result;
use bitvec::prelude::*;
use std::path::Path;

/// Message format version
const MESSAGE_FORMAT_VERSION: u8 = 1;

/// Header size in bytes
const HEADER_SIZE: usize = 8;

/// Encodes a message into an image using the BLTM steganography method
pub struct Encoder {
    /// The BLTM used for encoding
    bltm: BLTM3x3,
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Encoder {
    /// Create a new encoder with a 3x3 BLTM
    pub fn new() -> Self {
        Self {
            bltm: BLTM3x3::new(),
        }
    }

    /// Encode k bits of message into an RGB pixel using the BLTM algorithm
    ///
    /// # Arguments
    /// * `r` - Red component value (0-255)
    /// * `g` - Green component value (0-255)
    /// * `b` - Blue component value (0-255)
    /// * `message_bits` - k bits of the message to encode (in our case, 3 bits)
    ///
    /// # Returns
    /// * A tuple of the modified RGB values (r, g, b)
    pub fn encode_pixel(
        &self,
        r: u8,
        g: u8,
        b: u8,
        message_bits: &BitSlice<u8, Msb0>,
    ) -> (u8, u8, u8) {
        // Step 5-6: Extract LSBs to form cover vector vc
        let mut cover_vector = BitVec::<u8, Msb0>::new();
        cover_vector.push(r & 1 != 0); // LSB of R
        cover_vector.push(g & 1 != 0); // LSB of G
        cover_vector.push(b & 1 != 0); // LSB of B

        // Step 7: Find v = vc^T (transpose not needed for a single vector)
        // v = cover_vector already in the right orientation

        // Step 8-9: z = (A × v)^T
        let z = self.matrix_multiply(&cover_vector);

        // Step 10-11: Select message bits and compute δ = z ⊕ m
        let mut delta = BitVec::<u8, Msb0>::new();
        for i in 0..message_bits.len() {
            delta.push(z[i] ^ message_bits[i]);
        }

        // Step 12: Find Vn corresponding to δ
        let vn = self.bltm.lookup_vn(&delta);

        // Step 13: Compute stego-vector vs = vc ⊕ Vn
        let mut stego_vector = BitVec::<u8, Msb0>::new();
        for i in 0..cover_vector.len() {
            stego_vector.push(cover_vector[i] ^ vn[i]);
        }

        // Step 14: Modify RGB components based on vs
        let mut new_r = r;
        let mut new_g = g;
        let mut new_b = b;

        // Replace LSB of each component with corresponding bit from stego_vector
        if stego_vector[0] {
            new_r |= 1; // Set LSB to 1
        } else {
            new_r &= !1; // Set LSB to 0
        }

        if stego_vector[1] {
            new_g |= 1; // Set LSB to 1
        } else {
            new_g &= !1; // Set LSB to 0
        }

        if stego_vector[2] {
            new_b |= 1; // Set LSB to 1
        } else {
            new_b &= !1; // Set LSB to 0
        }

        (new_r, new_g, new_b)
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

    /// Encode an entire message into an image
    ///
    /// # Arguments
    /// * `cover_image` - The original image to embed the message into
    /// * `message` - The message bytes to embed
    ///
    /// # Returns
    /// * The stego image with the embedded message
    pub fn encode(&self, cover_image: StegoImage, message: &[u8]) -> Result<StegoImage> {
        // Calculate the maximum message size this image can hold
        let max_message_size = self.max_message_size(&cover_image);

        // Check if the message will fit (accounting for header)
        if message.len() > max_message_size {
            return Err(HideError::MessageTooLarge);
        }

        // Create a header containing metadata about the message
        let header = self.create_header(message.len() as u32)?;

        // Combine header and message
        let mut full_message = Vec::with_capacity(header.len() + message.len());
        full_message.extend_from_slice(&header);
        full_message.extend_from_slice(message);

        // Encode the full message (header + content)
        self.encode_message(cover_image, &full_message)
    }

    /// Create a header containing metadata about the message
    ///
    /// Header format (8 bytes total):
    /// - 1 byte: Message format version
    /// - 4 bytes: Message length (u32, big endian)
    /// - 3 bytes: Reserved for future use
    fn create_header(&self, message_length: u32) -> Result<[u8; HEADER_SIZE]> {
        let mut header = [0u8; HEADER_SIZE];

        // Set format version
        header[0] = MESSAGE_FORMAT_VERSION;

        // Set message length (big endian)
        header[1] = (message_length >> 24) as u8;
        header[2] = (message_length >> 16) as u8;
        header[3] = (message_length >> 8) as u8;
        header[4] = message_length as u8;

        // Reserved bytes are left as zeros

        Ok(header)
    }

    /// Encode a message into an image
    ///
    /// # Arguments
    /// * `image` - The cover image to encode the message into
    /// * `message` - The message to encode as bytes
    ///
    /// # Returns
    /// * The stego image with the encoded message
    pub fn encode_message(&self, mut image: StegoImage, message: &[u8]) -> Result<StegoImage> {
        // Convert the message to bits
        let message_bits = utils::bytes_to_bits(message);

        // Check if the message will fit in the image
        let max_bits = (image.width() * image.height() * 3) as usize;
        if message_bits.len() > max_bits {
            return Err(HideError::MessageTooLarge);
        }

        // Split message into 3-bit chunks for encoding
        let chunks = utils::split_bits(&message_bits, 3)?;

        // Track our position in the chunks
        let mut chunk_idx = 0;

        // Iterate through each pixel in the image
        for y in 0..image.height() {
            for x in 0..image.width() {
                // If we've encoded all chunks, we're done
                if chunk_idx >= chunks.len() {
                    return Ok(image);
                }

                // Get the current pixel
                let pixel = image.get_pixel_rgb(x, y)?;

                // Encode the current chunk into this pixel
                let (new_r, new_g, new_b) =
                    self.encode_pixel(pixel.0[0], pixel.0[1], pixel.0[2], &chunks[chunk_idx]);

                // Update the pixel with the encoded values
                image.set_pixel_rgb(x, y, image::Rgb([new_r, new_g, new_b]))?;

                // Move to the next chunk
                chunk_idx += 1;
            }
        }

        Ok(image)
    }

    /// Calculate the maximum message size that can be stored in an image
    ///
    /// # Arguments
    /// * `image` - The cover image
    ///
    /// # Returns
    /// * Maximum message size in bytes (accounting for header)
    pub fn max_message_size(&self, image: &StegoImage) -> usize {
        let total_pixels = image.width() * image.height();
        let total_bits = total_pixels * 3; // 3 bits per pixel (R,G,B)
        let total_bytes = total_bits as usize / 8;

        if total_bytes <= HEADER_SIZE {
            0 // Image too small to hold a header
        } else {
            total_bytes - HEADER_SIZE // Subtract header size
        }
    }

    /// Encode a message into an image and save the result
    ///
    /// # Arguments
    /// * `cover_image_path` - Path to the cover image
    /// * `message` - Message to encode
    /// * `output_path` - Path to save the stego image
    ///
    /// # Returns
    /// * Result indicating success or failure
    pub fn encode_file<P: AsRef<Path>>(
        &self,
        cover_image_path: P,
        message: &[u8],
        output_path: P,
    ) -> Result<()> {
        // Load the cover image
        let cover_image = StegoImage::from_file(cover_image_path)?;

        // Encode the message
        let stego_image = self.encode(cover_image, message)?;

        // Save the stego image
        stego_image.save(output_path)?;

        Ok(())
    }

    /// Get a reference to the BLTM used by this encoder
    pub fn bltm(&self) -> &BLTM3x3 {
        &self.bltm
    }
}

/// Create a new encoder with default settings
pub fn create_encoder() -> Encoder {
    Encoder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::img::create_rgb_image;
    use image::Rgb;

    #[test]
    fn test_encode_pixel_example() {
        let encoder = Encoder::new();

        // Input values from the example
        let r = 123; // (01111011)2
        let g = 127; // (01111111)2
        let b = 135; // (10000111)2

        // Message bits m = (110)2
        let message = bitvec![u8, Msb0; 1, 1, 0];

        // Encode the pixel
        let (new_r, new_g, new_b) = encoder.encode_pixel(r, g, b, &message);

        // Expected output from example:
        // Rs = 123 = (01111011)2 - unchanged
        // Gs = 126 = (01111110)2 - LSB changed from 1 to 0
        // Bs = 135 = (10000111)2 - unchanged
        assert_eq!(new_r, 123, "Red value should remain 123");
        assert_eq!(new_g, 126, "Green value should change to 126");
        assert_eq!(new_b, 135, "Blue value should remain 135");

        // Verify individual bits
        assert_eq!(new_r & 1, 1, "LSB of R should be 1");
        assert_eq!(new_g & 1, 0, "LSB of G should be 0");
        assert_eq!(new_b & 1, 1, "LSB of B should be 1");
    }

    #[test]
    fn test_matrix_multiply() {
        let encoder = Encoder::new();

        // Test multiplication with vector [1, 1, 1]
        let v = bitvec![u8, Msb0; 1, 1, 1];
        let result = encoder.matrix_multiply(&v);

        // Expected: A × [1, 1, 1]^T = [1, 0, 1]^T for a 3×3 lower triangular matrix
        let expected = bitvec![u8, Msb0; 1, 0, 1];
        assert_eq!(
            result, expected,
            "Matrix multiplication should yield [1, 0, 1]"
        );

        // Test with vector [0, 1, 0]
        let v = bitvec![u8, Msb0; 0, 1, 0];
        let result = encoder.matrix_multiply(&v);
        let expected = bitvec![u8, Msb0; 0, 1, 1];
        assert_eq!(
            result, expected,
            "Matrix multiplication should yield [0, 1, 1]"
        );
    }

    #[test]
    fn test_encode_full_message() {
        let encoder = Encoder::new();

        // Create a test image (10x10 = 100 pixels, can store up to 37 bytes)
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

        // Create a test message
        let message = b"Hello, steganography!";

        // Encode the message
        let stego_image = encoder.encode(image, message).unwrap();

        // The resulting image should have same dimensions
        assert_eq!(stego_image.width(), 10);
        assert_eq!(stego_image.height(), 10);

        // Check that pixels have been modified (only LSBs should change)
        let mut changes_count = 0;
        for y in 0..10 {
            for x in 0..10 {
                let original_r = (x * 20) as u8;
                let original_g = (y * 20) as u8;
                let original_b = ((x + y) * 10) as u8;

                let stego_pixel = stego_image.get_pixel_rgb(x, y).unwrap();

                // Check that only LSB might have changed
                if stego_pixel.0[0] != original_r {
                    assert_eq!(stego_pixel.0[0], original_r ^ 1);
                    changes_count += 1;
                }
                if stego_pixel.0[1] != original_g {
                    assert_eq!(stego_pixel.0[1], original_g ^ 1);
                    changes_count += 1;
                }
                if stego_pixel.0[2] != original_b {
                    assert_eq!(stego_pixel.0[2], original_b ^ 1);
                    changes_count += 1;
                }
            }
        }

        // Ensure some pixels were modified
        assert!(changes_count > 0);
    }

    #[test]
    fn test_message_too_large() {
        let encoder = Encoder::new();

        // Create a small image (2x2 = 4 pixels, can store 1 byte of message)
        let image = create_rgb_image(2, 2);

        // Try to encode a message that's too large
        let large_message = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let result = encoder.encode(image, &large_message);

        // Should fail with MessageTooLarge error
        assert!(result.is_err());
        match result {
            Err(HideError::MessageTooLarge) => (), // Expected
            _ => panic!("Expected MessageTooLarge error"),
        }
    }

    #[test]
    fn test_header_creation() {
        let encoder = Encoder::new();

        // Create a header for a message
        let header = encoder.create_header(1234).unwrap();

        // Check header format
        assert_eq!(header[0], MESSAGE_FORMAT_VERSION);

        // Check message length (big endian)
        assert_eq!(header[1], 0);
        assert_eq!(header[2], 0);
        assert_eq!(header[3], 4);
        assert_eq!(header[4], 210);

        // Reconstruct message length
        let msg_len = (header[1] as u32) << 24
            | (header[2] as u32) << 16
            | (header[3] as u32) << 8
            | (header[4] as u32);
        assert_eq!(msg_len, 1234);
    }

    #[test]
    fn test_max_message_size() {
        let encoder = Encoder::new();

        // Test various image sizes
        let test_cases = [
            (8, 8, 24 - HEADER_SIZE), // 64 pixels = 24 bytes - 8 bytes header = 16 bytes
            (10, 10, 37 - HEADER_SIZE), // 100 pixels = 37 bytes - 8 bytes header = 29 bytes
            (2, 2, 0),                // 4 pixels = 1 byte (too small for header + message)
        ];

        for (width, height, expected_size) in test_cases.iter() {
            let image = create_rgb_image(*width, *height);
            let max_size = encoder.max_message_size(&image);
            assert_eq!(
                max_size, *expected_size,
                "Wrong max message size for {}x{} image",
                width, height
            );
        }
    }
}
