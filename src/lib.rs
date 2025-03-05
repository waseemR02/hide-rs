//! # hide-rs
//!
//! `hide-rs` is a steganography library that implements the Binary Linear Transformation Matrix
//! (BLTM) method for hiding messages within images. This library provides functionality to
//! encode messages into images and decode them back without visible changes to the image.

mod bltm;
mod encoder;
mod decoder;
mod error;
mod utils;

/// The result type returned by functions in this library.
pub type Result<T> = std::result::Result<T, error::HideError>;
