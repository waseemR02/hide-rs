//! Data models for the REST API

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::PathBuf;

/// Maximum allowed message length (in bytes) to prevent abuse
pub const MAX_MESSAGE_LENGTH: usize = 1024 * 1024; // 1MB

/// Maximum allowed image size (in bytes) to prevent abuse
pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Function to generate a new UUID v4
fn generate_uuid_v4() -> Uuid {
    Uuid::new_v4()
}

/// Base request payload that all requests will contain
#[derive(Debug, Serialize, Deserialize)]
pub struct BaseRequest {
    /// Optional request ID for tracking
    #[serde(default = "generate_uuid_v4")]
    pub request_id: Uuid,
}

/// Request to encode a message in an image
#[derive(Debug, Serialize, Deserialize)]
pub struct EncodeRequest {
    #[serde(flatten)]
    pub base: BaseRequest,
    
    /// Message to hide in the image (text format)
    #[serde(default)]
    pub message: Option<String>,
    
    /// Binary message to hide in the image (base64 encoded)
    #[serde(default)]
    pub binary_message: Option<String>,
    
    /// Options for the encoding process
    #[serde(default)]
    pub options: EncodeOptions,
}

/// Options for the encoding process
#[derive(Debug, Serialize, Deserialize)]
pub struct EncodeOptions {
    /// Output image format (png, jpeg, etc.)
    #[serde(default = "default_output_format")]
    pub output_format: String,
    
    /// JPEG quality (0-100) if output format is JPEG
    #[serde(default = "default_jpeg_quality")]
    pub jpeg_quality: u8,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            output_format: default_output_format(),
            jpeg_quality: default_jpeg_quality(),
        }
    }
}

fn default_output_format() -> String {
    "png".to_string()
}

fn default_jpeg_quality() -> u8 {
    90
}

/// Response for successful encoding
#[derive(Debug, Serialize)]
pub struct EncodeResponse {
    /// Request ID from the original request
    pub request_id: Uuid,
    
    /// Status of the operation
    pub status: String,
    
    /// ID of the encoded image for retrieval
    pub image_id: Uuid,
    
    /// URL path to download the encoded image
    pub download_url: String,
    
    /// Metadata about the encoded image
    pub metadata: ImageMetadata,
}

/// Response for successful decoding
#[derive(Debug, Serialize)]
pub struct DecodeResponse {
    /// Request ID from the original request
    pub request_id: Uuid,
    
    /// Status of the operation
    pub status: String,
    
    /// The decoded message (if it's valid UTF-8 text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    
    /// The decoded binary message (base64 encoded)
    pub binary_message: String,
    
    /// Length of the decoded message in bytes
    pub message_length: usize,
}

/// Metadata about an image
#[derive(Debug, Serialize)]
pub struct ImageMetadata {
    /// Width of the image in pixels
    pub width: u32,
    
    /// Height of the image in pixels
    pub height: u32,
    
    /// Format of the image (PNG, JPEG, etc.)
    pub format: String,
    
    /// Size of the image in bytes
    pub size_bytes: usize,
    
    /// Maximum message size that could be embedded in this image
    pub max_message_bytes: usize,
    
    /// Actual message size that was embedded (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_message_bytes: Option<usize>,
}

/// Information about a stored image
#[derive(Debug, Serialize)]
pub struct ImageInfo {
    /// Unique ID of the image
    pub id: Uuid,
    
    /// Path to the image file on disk
    #[serde(skip_serializing)]
    pub file_path: PathBuf,
    
    /// URL path to download the image
    pub download_url: String,
    
    /// Metadata about the image
    pub metadata: ImageMetadata,
    
    /// When the image was created/uploaded
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Error response for API requests
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Request ID from the original request
    pub request_id: Uuid,
    
    /// Status of the operation (always "error")
    pub status: String,
    
    /// Error code
    pub error_code: String,
    
    /// Human-readable error message
    pub message: String,
    
    /// Additional details about the error (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(request_id: Uuid, error_code: &str, message: &str) -> Self {
        Self {
            request_id,
            status: "error".to_string(),
            error_code: error_code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }
    
    /// Add details to the error response
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Error codes used in API responses
pub mod error_codes {
    pub const VALIDATION_ERROR: &str = "validation_error";
    pub const IMAGE_TOO_LARGE: &str = "image_too_large";
    pub const MESSAGE_TOO_LARGE: &str = "message_too_large";
    pub const INVALID_IMAGE: &str = "invalid_image";
    pub const NO_MESSAGE_FOUND: &str = "no_message_found";
    pub const INTERNAL_ERROR: &str = "internal_error";
    pub const NOT_FOUND: &str = "not_found";
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_string, from_str};
    
    #[test]
    fn test_encode_request_serialization() {
        // Create a request with text message
        let req = EncodeRequest {
            base: BaseRequest {
                request_id: Uuid::new_v4(),
            },
            message: Some("Hello, world!".to_string()),
            binary_message: None,
            options: EncodeOptions::default(),
        };
        
        // Serialize to JSON
        let json_str = to_string(&req).expect("Failed to serialize EncodeRequest");
        
        // Check that it contains the message
        assert!(json_str.contains("Hello, world!"));
        assert!(json_str.contains("request_id"));
    }
    
    #[test]
    fn test_encode_request_deserialization() {
        // Create a JSON string
        let json_str = r#"
        {
            "request_id": "550e8400-e29b-41d4-a716-446655440000",
            "message": "Hello, world!",
            "options": {
                "output_format": "jpeg",
                "jpeg_quality": 85
            }
        }
        "#;
        
        // Deserialize from JSON
        let req: EncodeRequest = from_str(json_str).expect("Failed to deserialize EncodeRequest");
        
        // Check values
        assert_eq!(req.base.request_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(req.message, Some("Hello, world!".to_string()));
        assert_eq!(req.binary_message, None);
        assert_eq!(req.options.output_format, "jpeg");
        assert_eq!(req.options.jpeg_quality, 85);
    }
    
    #[test]
    fn test_encode_response_serialization() {
        // Create a response
        let res = EncodeResponse {
            request_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            status: "success".to_string(),
            image_id: Uuid::parse_str("650e8400-e29b-41d4-a716-446655440001").unwrap(),
            download_url: "/api/images/650e8400-e29b-41d4-a716-446655440001".to_string(),
            metadata: ImageMetadata {
                width: 800,
                height: 600,
                format: "png".to_string(),
                size_bytes: 12345,
                max_message_bytes: 1000,
                embedded_message_bytes: Some(100),
            },
        };
        
        // Serialize to JSON
        let json_str = to_string(&res).expect("Failed to serialize EncodeResponse");
        
        // Check that it contains the expected fields
        assert!(json_str.contains("success"));
        assert!(json_str.contains("650e8400-e29b-41d4-a716-446655440001"));
        assert!(json_str.contains("800"));
        assert!(json_str.contains("600"));
        assert!(json_str.contains("100"));
    }
    
    #[test]
    fn test_error_response_creation() {
        // Create an error response
        let err = ErrorResponse::new(
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            error_codes::VALIDATION_ERROR,
            "Invalid input"
        ).with_details(json!({
            "field": "message",
            "reason": "Message is too large"
        }));
        
        // Check values
        assert_eq!(err.status, "error");
        assert_eq!(err.error_code, error_codes::VALIDATION_ERROR);
        assert_eq!(err.message, "Invalid input");
        assert!(err.details.is_some());
        
        // Serialize to JSON
        let json_str = to_string(&err).expect("Failed to serialize ErrorResponse");
        assert!(json_str.contains("validation_error"));
        assert!(json_str.contains("Invalid input"));
        assert!(json_str.contains("message"));
        assert!(json_str.contains("Message is too large"));
    }
    
    #[test]
    fn test_default_values() {
        // Create a minimal JSON string (missing optional fields)
        let json_str = r#"
        {
            "request_id": "550e8400-e29b-41d4-a716-446655440000"
        }
        "#;
        
        // Deserialize from JSON
        let req: EncodeRequest = from_str(json_str).expect("Failed to deserialize minimal EncodeRequest");
        
        // Check default values
        assert_eq!(req.message, None);
        assert_eq!(req.binary_message, None);
        assert_eq!(req.options.output_format, "png");
        assert_eq!(req.options.jpeg_quality, 90);
    }
    
    #[test]
    fn test_missing_request_id_generates_new_one() {
        // Create a JSON string without request_id
        let json_str = r#"
        {
            "message": "Hello, world!"
        }
        "#;
        
        // Deserialize from JSON
        let req: EncodeRequest = from_str(json_str).expect("Failed to deserialize EncodeRequest without request_id");
        
        // Check that request_id was generated
        assert!(req.base.request_id.to_string().len() > 0);
    }
}
