use crate::error::HideError;
use crate::Result;
use image::{DynamicImage, GenericImageView, ImageBuffer, Pixel, Rgb, Rgba};
use std::path::Path;

/// Represents an image that can be used for steganography
#[derive(Clone)]
pub struct StegoImage {
    /// The underlying image data
    image: DynamicImage,
    /// Whether the image has been modified
    modified: bool,
}

impl StegoImage {
    /// Load an image from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let image = image::open(path)?;

        Ok(Self {
            image,
            modified: false,
        })
    }

    /// Create a new StegoImage from a DynamicImage
    pub fn from_dynamic_image(image: DynamicImage) -> Self {
        Self {
            image,
            modified: false,
        }
    }

    /// Create a new RGB image with the specified dimensions
    pub fn new_rgb(width: u32, height: u32) -> Self {
        let img_buffer = ImageBuffer::new(width, height);
        let image = DynamicImage::ImageRgb8(img_buffer);

        Self {
            image,
            modified: false,
        }
    }

    /// Get the image width
    pub fn width(&self) -> u32 {
        self.image.width()
    }

    /// Get the image height
    pub fn height(&self) -> u32 {
        self.image.height()
    }

    /// Get the image dimensions (width, height)
    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    /// Get the underlying dynamic image
    pub fn inner(&self) -> &DynamicImage {
        &self.image
    }

    /// Get a mutable reference to the underlying dynamic image
    pub fn inner_mut(&mut self) -> &mut DynamicImage {
        self.modified = true;
        &mut self.image
    }

    /// Get the RGB value of a pixel at the given coordinates
    pub fn get_pixel_rgb(&self, x: u32, y: u32) -> Result<Rgb<u8>> {
        if x >= self.width() || y >= self.height() {
            return Err(HideError::InvalidParameters(format!(
                "Coordinates ({}, {}) out of image bounds ({}x{})",
                x,
                y,
                self.width(),
                self.height()
            )));
        }

        Ok(self.image.get_pixel(x, y).to_rgb())
    }

    /// Set the RGB value of a pixel at the given coordinates
    pub fn set_pixel_rgb(&mut self, x: u32, y: u32, pixel: Rgb<u8>) -> Result<()> {
        if x >= self.width() || y >= self.height() {
            return Err(HideError::InvalidParameters(format!(
                "Coordinates ({}, {}) out of image bounds ({}x{})",
                x,
                y,
                self.width(),
                self.height()
            )));
        }

        // Convert to the appropriate image format if needed
        match self.image {
            DynamicImage::ImageRgb8(ref mut img) => {
                img.put_pixel(x, y, pixel);
            }
            _ => {
                // For other formats, we need to convert to RGB first
                let mut rgb_img = self.image.to_rgb8();
                rgb_img.put_pixel(x, y, pixel);
                self.image = DynamicImage::ImageRgb8(rgb_img);
            }
        }

        self.modified = true;
        Ok(())
    }

    /// Get the RGBA value of a pixel at the given coordinates
    pub fn get_pixel_rgba(&self, x: u32, y: u32) -> Result<Rgba<u8>> {
        if x >= self.width() || y >= self.height() {
            return Err(HideError::InvalidParameters(format!(
                "Coordinates ({}, {}) out of image bounds ({}x{})",
                x,
                y,
                self.width(),
                self.height()
            )));
        }

        Ok(self.image.get_pixel(x, y))
    }

    /// Set the RGBA value of a pixel at the given coordinates
    pub fn set_pixel_rgba(&mut self, x: u32, y: u32, pixel: Rgba<u8>) -> Result<()> {
        if x >= self.width() || y >= self.height() {
            return Err(HideError::InvalidParameters(format!(
                "Coordinates ({}, {}) out of image bounds ({}x{})",
                x,
                y,
                self.width(),
                self.height()
            )));
        }

        // Convert to the appropriate image format if needed
        match self.image {
            DynamicImage::ImageRgba8(ref mut img) => {
                img.put_pixel(x, y, pixel);
            }
            _ => {
                // For other formats, we need to convert to RGBA first
                let mut rgba_img = self.image.to_rgba8();
                rgba_img.put_pixel(x, y, pixel);
                self.image = DynamicImage::ImageRgba8(rgba_img);
            }
        }

        self.modified = true;
        Ok(())
    }

    /// Modify the least significant bit of a color channel
    pub fn set_lsb(&mut self, x: u32, y: u32, channel: usize, bit: bool) -> Result<()> {
        if channel > 2 {
            return Err(HideError::InvalidParameters(format!(
                "Invalid color channel index: {}. Must be 0 (R), 1 (G), or 2 (B)",
                channel
            )));
        }

        let mut pixel = self.get_pixel_rgb(x, y)?;

        if bit {
            pixel.0[channel] |= 1; // Set LSB to 1
        } else {
            pixel.0[channel] &= !1; // Set LSB to 0
        }

        self.set_pixel_rgb(x, y, pixel)
    }

    /// Get the least significant bit of a color channel
    pub fn get_lsb(&self, x: u32, y: u32, channel: usize) -> Result<bool> {
        if channel > 2 {
            return Err(HideError::InvalidParameters(format!(
                "Invalid color channel index: {}. Must be 0 (R), 1 (G), or 2 (B)",
                channel
            )));
        }

        let pixel = self.get_pixel_rgb(x, y)?;
        Ok((pixel.0[channel] & 1) == 1)
    }

    /// Save the image to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.image.save(path)?;
        Ok(())
    }

    /// Convert the image to RGB format
    pub fn to_rgb(&mut self) {
        if !matches!(self.image, DynamicImage::ImageRgb8(_)) {
            let rgb_image = self.image.to_rgb8();
            self.image = DynamicImage::ImageRgb8(rgb_image);
            self.modified = true;
        }
    }

    /// Convert the image to RGBA format
    pub fn to_rgba(&mut self) {
        if !matches!(self.image, DynamicImage::ImageRgba8(_)) {
            let rgba_image = self.image.to_rgba8();
            self.image = DynamicImage::ImageRgba8(rgba_image);
            self.modified = true;
        }
    }

    /// Check if the image has been modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Calculate the maximum message size (in bytes) that can be stored in this image
    /// Each pixel can store 3 bits (one in each RGB channel)
    pub fn max_message_size(&self) -> usize {
        let total_pixels = self.width() * self.height();
        let total_bits = total_pixels * 3; // 3 bits per pixel (R,G,B)
        total_bits as usize / 8 // convert to bytes
    }
}

/// Create a new blank RGB image with the specified dimensions
pub fn create_rgb_image(width: u32, height: u32) -> StegoImage {
    StegoImage::new_rgb(width, height)
}

/// Load an image from a file
pub fn load_image<P: AsRef<Path>>(path: P) -> Result<StegoImage> {
    StegoImage::from_file(path)
}

/// Save an image to a file
pub fn save_image<P: AsRef<Path>>(image: &StegoImage, path: P) -> Result<()> {
    image.save(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageFormat;
    use std::io::Cursor;

    fn create_test_image() -> StegoImage {
        let img_data = vec![
            255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 0, 255, 255, 255, 0, 255, 128, 128, 128,
            255, 255, 255, 0, 0, 0, 100, 100, 100, 50, 150, 250, 200, 50, 150,
        ];

        let img_buffer = ImageBuffer::from_raw(4, 3, img_data).unwrap();
        StegoImage::from_dynamic_image(DynamicImage::ImageRgb8(img_buffer))
    }

    #[test]
    fn test_image_dimensions() {
        let img = create_test_image();
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 3);
        assert_eq!(img.dimensions(), (4, 3));
    }

    #[test]
    fn test_get_set_pixel_rgb() {
        let mut img = create_test_image();

        // Test getting a pixel
        let pixel = img.get_pixel_rgb(0, 0).unwrap();
        assert_eq!(pixel.0, [255, 0, 0]);

        // Test setting a pixel
        img.set_pixel_rgb(0, 0, Rgb([10, 20, 30])).unwrap();
        let updated_pixel = img.get_pixel_rgb(0, 0).unwrap();
        assert_eq!(updated_pixel.0, [10, 20, 30]);

        // Test out of bounds access
        assert!(img.get_pixel_rgb(10, 10).is_err());
        assert!(img.set_pixel_rgb(10, 10, Rgb([0, 0, 0])).is_err());
    }

    #[test]
    fn test_lsb_operations() {
        let mut img = create_test_image();

        // Test getting LSB
        assert!(img.get_lsb(0, 0, 0).unwrap()); // Red: 255 (odd)
        assert!(!img.get_lsb(0, 0, 1).unwrap()); // Green: 0 (even)
        assert!(!img.get_lsb(0, 0, 2).unwrap()); // Blue: 0 (even)

        // Test setting LSB
        img.set_lsb(0, 0, 0, false).unwrap(); // Set red LSB to 0
        assert!(!img.get_lsb(0, 0, 0).unwrap());
        assert_eq!(img.get_pixel_rgb(0, 0).unwrap().0[0], 254); // 255 -> 254

        img.set_lsb(0, 0, 1, true).unwrap(); // Set green LSB to 1
        assert!(img.get_lsb(0, 0, 1).unwrap());
        assert_eq!(img.get_pixel_rgb(0, 0).unwrap().0[1], 1); // 0 -> 1

        // Test invalid channel
        assert!(img.get_lsb(0, 0, 3).is_err());
        assert!(img.set_lsb(0, 0, 3, true).is_err());
    }

    #[test]
    fn test_save_load_image() {
        let img = create_test_image();

        // Save to memory buffer
        let mut buffer = Cursor::new(Vec::new());
        img.inner().write_to(&mut buffer, ImageFormat::Png).unwrap();

        // Load from memory buffer
        buffer.set_position(0);
        let loaded = image::load(buffer, ImageFormat::Png).unwrap();
        let loaded_img = StegoImage::from_dynamic_image(loaded);

        // Verify dimensions match
        assert_eq!(loaded_img.dimensions(), img.dimensions());

        // Verify some pixel values
        assert_eq!(
            loaded_img.get_pixel_rgb(0, 0).unwrap().0,
            img.get_pixel_rgb(0, 0).unwrap().0
        );
        assert_eq!(
            loaded_img.get_pixel_rgb(1, 1).unwrap().0,
            img.get_pixel_rgb(1, 1).unwrap().0
        );
    }

    #[test]
    fn test_max_message_size() {
        let img = create_test_image();
        // 4×3 image = 12 pixels × 3 bits per pixel = 36 bits = 4.5 bytes
        assert_eq!(img.max_message_size(), 4);
    }
}
