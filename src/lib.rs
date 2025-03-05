//! # hide-rs
//!
//! `hide-rs` is a steganography library that implements the Binary Linear Transformation Matrix
//! (BLTM) method for hiding messages within images. This library provides functionality to
//! encode messages into images and decode them back without visible changes to the image.

mod bltm;
mod decoder;
mod encoder;
mod error;
mod img;
mod utils;

// Re-export items for public API
pub use error::HideError;
pub use img::{create_rgb_image, load_image, save_image, StegoImage};
// Re-export the Rgb type from the image crate for convenient use
pub use image::Rgb;

/// The result type returned by functions in this library.
pub type Result<T> = std::result::Result<T, error::HideError>;

/// Version of the hide-rs library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_operations() {
        // Create a small test image
        let mut img = create_rgb_image(2, 2);

        // Set some pixel values
        img.set_pixel_rgb(0, 0, Rgb([255, 0, 0])).unwrap();
        img.set_pixel_rgb(1, 0, Rgb([0, 255, 0])).unwrap();
        img.set_pixel_rgb(0, 1, Rgb([0, 0, 255])).unwrap();
        img.set_pixel_rgb(1, 1, Rgb([255, 255, 255])).unwrap();

        // Verify pixel values
        assert_eq!(img.get_pixel_rgb(0, 0).unwrap().0, [255, 0, 0]);
        assert_eq!(img.get_pixel_rgb(1, 0).unwrap().0, [0, 255, 0]);
        assert_eq!(img.get_pixel_rgb(0, 1).unwrap().0, [0, 0, 255]);
        assert_eq!(img.get_pixel_rgb(1, 1).unwrap().0, [255, 255, 255]);
    }
}
