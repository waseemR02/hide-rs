use thiserror::Error;

/// Error types for the hide-rs library
#[derive(Error, Debug)]
pub enum HideError {
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Image processing errors
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    /// Message too large for the given image
    #[error("Message is too large for the given image")]
    MessageTooLarge,

    /// No message found in the image
    #[error("No message found in the image")]
    NoMessageFound,

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Matrix error
    #[error("Matrix error: {0}")]
    MatrixError(String),
}
