//! # hide-rs
//!
//! `hide-rs` is a steganography library that implements the Binary Linear Transformation Matrix
//! (BLTM) method for hiding messages within images. This library provides functionality to
//! encode messages into images and decode them back without visible changes to the image.

pub mod api;
pub mod bltm;
pub mod decoder;
pub mod encoder;
pub mod error;
pub mod img;
pub mod raw_decoder;
pub mod utils;

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
        let mut img = img::create_rgb_image(2, 2);

        // Set some pixel values
        img.set_pixel_rgb(0, 0, image::Rgb([255, 0, 0])).unwrap();
        img.set_pixel_rgb(1, 0, image::Rgb([0, 255, 0])).unwrap();
        img.set_pixel_rgb(0, 1, image::Rgb([0, 0, 255])).unwrap();
        img.set_pixel_rgb(1, 1, image::Rgb([255, 255, 255]))
            .unwrap();

        // Verify pixel values
        assert_eq!(img.get_pixel_rgb(0, 0).unwrap().0, [255, 0, 0]);
        assert_eq!(img.get_pixel_rgb(1, 0).unwrap().0, [0, 255, 0]);
        assert_eq!(img.get_pixel_rgb(0, 1).unwrap().0, [0, 0, 255]);
        assert_eq!(img.get_pixel_rgb(1, 1).unwrap().0, [255, 255, 255]);
    }
}
