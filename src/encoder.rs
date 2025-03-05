//! Encoding functionality for steganography

use crate::bltm::BLTM3x3;
use crate::error::HideError;
use crate::img::StegoImage;
use crate::utils;
use crate::Result;
use bitvec::prelude::*;

/// Encodes a message into an image using the BLTM steganography method
pub struct Encoder {
    /// The BLTM used for encoding
    bltm: BLTM3x3,
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
                    break;
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

            if chunk_idx >= chunks.len() {
                break;
            }
        }

        Ok(image)
    }

    /// Get a reference to the BLTM used by this encoder
    pub fn bltm(&self) -> &BLTM3x3 {
        &self.bltm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    

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
}
