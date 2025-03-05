//! Request handlers for the REST API

use crate::api::models::*;
use crate::encoder::create_encoder;
use crate::decoder::create_decoder;
use crate::error::HideError;
use crate::img::StegoImage;

use actix_web::{HttpResponse, Error};
use actix_multipart::Multipart;
use futures::StreamExt;
use uuid::Uuid;
use std::io::Write;
use std::path::{Path, PathBuf};
use log::{info, error, warn};
use std::fs;
use serde::{Serialize, Deserialize};
use mime_guess::from_path;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Stores temporary files related to a request
#[derive(Debug)]
pub struct RequestFiles {
    /// Directory where files are stored
    pub base_dir: PathBuf,
    
    /// ID of the request
    pub request_id: Uuid,
    
    /// Paths to any files created
    pub file_paths: Vec<PathBuf>,
}

impl RequestFiles {
    /// Create a new RequestFiles instance
    pub fn new(base_dir: impl AsRef<Path>, request_id: Uuid) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            request_id,
            file_paths: Vec::new(),
        }
    }
    
    /// Create a file in the temporary directory
    pub fn create_file(&mut self, filename: &str) -> std::io::Result<(PathBuf, fs::File)> {
        // Create directory if it doesn't exist
        let dir_path = self.base_dir.join(self.request_id.to_string());
        fs::create_dir_all(&dir_path)?;
        
        // Create file
        let file_path = dir_path.join(sanitize_filename::sanitize(filename));
        let file = fs::File::create(&file_path)?;
        
        // Remember the file path
        self.file_paths.push(file_path.clone());
        
        Ok((file_path, file))
    }
    
    /// Clean up all files created for this request
    pub fn cleanup(&self) {
        // Remove the entire directory
        let dir_path = self.base_dir.join(self.request_id.to_string());
        if let Err(e) = fs::remove_dir_all(&dir_path) {
            warn!("Failed to clean up request files for {}: {}", self.request_id, e);
        }
    }
}

impl Drop for RequestFiles {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Convert HideError to ErrorResponse
pub fn hide_error_to_response(err: HideError, request_id: Uuid) -> ErrorResponse {
    match err {
        HideError::MessageTooLarge => ErrorResponse::new(
            request_id,
            error_codes::MESSAGE_TOO_LARGE,
            "Message is too large for the given image"
        ),
        HideError::NoMessageFound => ErrorResponse::new(
            request_id,
            error_codes::NO_MESSAGE_FOUND,
            "No hidden message found in the image"
        ),
        HideError::Image(e) => ErrorResponse::new(
            request_id,
            error_codes::INVALID_IMAGE,
            &format!("Invalid image: {}", e)
        ),
        HideError::InvalidParameters(msg) => ErrorResponse::new(
            request_id,
            error_codes::VALIDATION_ERROR,
            &msg
        ),
        _ => ErrorResponse::new(
            request_id,
            error_codes::INTERNAL_ERROR,
            "An internal error occurred"
        ),
    }
}

/// Extract image metadata
pub fn extract_image_metadata(image: &StegoImage) -> ImageMetadata {
    let encoder = create_encoder();
    let max_message_bytes = encoder.max_message_size(image);
    
    ImageMetadata {
        width: image.width(),
        height: image.height(),
        format: "png".to_string(), // Always saved as PNG
        size_bytes: 0, // Will be updated after saving
        max_message_bytes,
        embedded_message_bytes: None,
    }
}

/// Process a multipart form submission for image encoding
pub async fn process_encode_form(mut payload: Multipart, upload_dir: &Path) -> Result<HttpResponse, Error> {
    info!("Processing encode form submission");
    
    let request_id = Uuid::new_v4();
    let mut files = RequestFiles::new(upload_dir, request_id);
    
    let mut cover_image_path: Option<PathBuf> = None;
    let mut message: Option<String> = None;
    let mut message_file_content: Option<Vec<u8>> = None;
    let mut options = EncodeOptions::default();
    
    // Process multipart form data
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => {
                error!("Error getting multipart field: {}", e);
                return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                    request_id,
                    error_codes::VALIDATION_ERROR,
                    &format!("Invalid form data: {}", e)
                )));
            }
        };
        
        // Fix: Use content_disposition() as Option and then unwrap it safely
        let content_disposition = field.content_disposition();
        let field_name = content_disposition
            .and_then(|cd| cd.get_name())
            .unwrap_or("")
            .to_string();
        
        match field_name.as_str() {
            "cover_image" => {
                // Fix: Use content_disposition() as Option and then get the filename safely
                let filename = content_disposition
                    .and_then(|cd| cd.get_filename())
                    .unwrap_or("cover_image.png")
                    .to_string();
                
                // Create a file to save the uploaded image
                let (path, mut file) = match files.create_file(&filename) {
                    Ok((p, f)) => (p, f),
                    Err(e) => {
                        error!("Failed to create file: {}", e);
                        return Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                            request_id,
                            error_codes::INTERNAL_ERROR,
                            "Failed to process uploaded file"
                        )));
                    }
                };
                
                // Save the file
                let mut size: usize = 0;
                while let Some(chunk) = field.next().await {
                    let data = match chunk {
                        Ok(d) => d,
                        Err(e) => {
                            error!("Error reading multipart chunk: {}", e);
                            return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                                request_id,
                                error_codes::VALIDATION_ERROR,
                                &format!("Error reading upload: {}", e)
                            )));
                        }
                    };
                    
                    size += data.len();
                    if size > MAX_IMAGE_SIZE {
                        return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                            request_id,
                            error_codes::IMAGE_TOO_LARGE,
                            &format!("Image exceeds maximum size of {} bytes", MAX_IMAGE_SIZE)
                        )));
                    }
                    
                    // Write chunk to file
                    if let Err(e) = file.write_all(&data) {
                        error!("Error writing to file: {}", e);
                        return Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                            request_id,
                            error_codes::INTERNAL_ERROR,
                            "Failed to save uploaded file"
                        )));
                    }
                }
                
                cover_image_path = Some(path);
            },
            "message" => {
                // Read the message content
                let mut content = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = match chunk {
                        Ok(d) => d,
                        Err(e) => {
                            error!("Error reading message: {}", e);
                            return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                                request_id,
                                error_codes::VALIDATION_ERROR,
                                &format!("Error reading message: {}", e)
                            )));
                        }
                    };
                    
                    if content.len() + data.len() > MAX_MESSAGE_LENGTH {
                        return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                            request_id,
                            error_codes::MESSAGE_TOO_LARGE,
                            &format!("Message exceeds maximum size of {} bytes", MAX_MESSAGE_LENGTH)
                        )));
                    }
                    
                    content.extend_from_slice(&data);
                }
                
                message = match String::from_utf8(content.clone()) {
                    Ok(s) => Some(s),
                    Err(_) => {
                        // Store as binary if not valid UTF-8
                        message_file_content = Some(content);
                        None
                    }
                };
            },
            "message_file" => {
                // Fix: Use content_disposition() as Option and then get the filename safely
                let _filename = content_disposition
                    .and_then(|cd| cd.get_filename())
                    .unwrap_or("message.txt")
                    .to_string();
                
                // Read the file content
                let mut content = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = match chunk {
                        Ok(d) => d,
                        Err(e) => {
                            error!("Error reading message file: {}", e);
                            return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                                request_id,
                                error_codes::VALIDATION_ERROR,
                                &format!("Error reading message file: {}", e)
                            )));
                        }
                    };
                    
                    if content.len() + data.len() > MAX_MESSAGE_LENGTH {
                        return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                            request_id,
                            error_codes::MESSAGE_TOO_LARGE,
                            &format!("Message exceeds maximum size of {} bytes", MAX_MESSAGE_LENGTH)
                        )));
                    }
                    
                    content.extend_from_slice(&data);
                }
                
                message_file_content = Some(content);
            },
            "output_format" => {
                // Read the output format
                let mut content = Vec::new();
                while let Some(chunk) = field.next().await {
                    content.extend_from_slice(&match chunk {
                        Ok(d) => d,
                        Err(_) => continue,
                    });
                }
                
                if let Ok(format) = String::from_utf8(content) {
                    let format = format.trim().to_lowercase();
                    if format == "png" || format == "jpeg" || format == "jpg" {
                        options.output_format = format;
                    }
                }
            },
            "jpeg_quality" => {
                // Read the JPEG quality
                let mut content = Vec::new();
                while let Some(chunk) = field.next().await {
                    content.extend_from_slice(&match chunk {
                        Ok(d) => d,
                        Err(_) => continue,
                    });
                }
                
                if let Ok(quality_str) = String::from_utf8(content) {
                    if let Ok(quality) = quality_str.trim().parse::<u8>() {
                        options.jpeg_quality = quality.min(100);
                    }
                }
            },
            _ => {
                // Skip unknown fields
                while let Some(_) = field.next().await {}
            }
        }
    }
    
    // Ensure we have a cover image
    let cover_image_path = match cover_image_path {
        Some(path) => path,
        None => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                request_id,
                error_codes::VALIDATION_ERROR,
                "Missing cover image"
            )));
        }
    };
    
    // Ensure we have a message (either text or file)
    let message_content = match (&message, &message_file_content) {
        (Some(text), _) => text.as_bytes().to_vec(),
        (_, Some(binary)) => binary.clone(),
        _ => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                request_id,
                error_codes::VALIDATION_ERROR,
                "Missing message content"
            )));
        }
    };
    
    // Load the cover image
    let cover_image = match StegoImage::from_file(&cover_image_path) {
        Ok(img) => img,
        Err(e) => {
            error!("Failed to load cover image: {}", e);
            return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                request_id,
                error_codes::INVALID_IMAGE,
                &format!("Failed to load cover image: {}", e)
            )));
        }
    };
    
    // Create the encoder
    let encoder = create_encoder();
    
    // Check if the message will fit
    let max_message_size = encoder.max_message_size(&cover_image);
    if message_content.len() > max_message_size {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
            request_id,
            error_codes::MESSAGE_TOO_LARGE,
            &format!(
                "Message is too large ({} bytes) for this image (max {} bytes)",
                message_content.len(),
                max_message_size
            )
        )));
    }
    
    // Encode the message
    let stego_image = match encoder.encode(cover_image, &message_content) {
        Ok(img) => img,
        Err(e) => {
            error!("Failed to encode message: {:?}", e);
            return Ok(HttpResponse::BadRequest().json(hide_error_to_response(e, request_id)));
        }
    };
    
    // Generate a unique ID for the stego image
    let image_id = Uuid::new_v4();
    
    // Save the stego image
    let stego_image_path = upload_dir.join(image_id.to_string() + ".png");
    if let Err(e) = stego_image.save(&stego_image_path) {
        error!("Failed to save stego image: {}", e);
        return Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
            request_id,
            error_codes::INTERNAL_ERROR,
            "Failed to save encoded image"
        )));
    }
    
    // Get the file size
    let size_bytes = match fs::metadata(&stego_image_path) {
        Ok(metadata) => metadata.len() as usize,
        Err(_) => 0,
    };
    
    // Extract metadata
    let mut metadata = extract_image_metadata(&stego_image);
    metadata.size_bytes = size_bytes;
    metadata.embedded_message_bytes = Some(message_content.len());
    
    // Create the response
    let response = EncodeResponse {
        request_id,
        status: "success".to_string(),
        image_id,
        download_url: format!("/api/images/{}", image_id),
        metadata,
    };
    
    Ok(HttpResponse::Ok().json(response))
}

/// Application state shared across requests
#[derive(Clone)]
pub struct AppState {
    /// Server configuration
    pub config: ServerConfig,
}

/// Server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Host address to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Temporary directory for file uploads
    pub upload_dir: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            upload_dir: "./tmp".to_string(),
        }
    }
}

/// Response for health check endpoint
#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Serve an encoded image file
pub async fn serve_encoded_image(image_id: String, upload_dir: &Path) -> Result<HttpResponse, Error> {
    // Validate the image ID format (basic security check)
    if !image_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Ok(HttpResponse::BadRequest().body("Invalid image ID format"));
    }
    
    // Construct the image path
    let image_path = upload_dir.join(format!("{}.png", image_id));
    
    // Check if the file exists
    if !image_path.exists() {
        return Ok(HttpResponse::NotFound().body("Image not found"));
    }
    
    // Read the file
    let file_data = match std::fs::read(&image_path) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to read image file: {}", e);
            return Ok(HttpResponse::InternalServerError().body("Failed to read image file"));
        }
    };
    
    // Determine content type based on file extension
    let content_type = from_path(&image_path)
        .first_or_octet_stream()
        .to_string();
    
    // Return the image with appropriate headers
    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .append_header(("Content-Disposition", format!("inline; filename=\"{}.png\"", image_id)))
        .body(file_data))
}

/// Process a multipart form submission for image decoding
pub async fn process_decode_form(mut payload: Multipart, upload_dir: &Path) -> Result<HttpResponse, Error> {
    info!("Processing decode form submission");
    
    let request_id = Uuid::new_v4();
    let mut files = RequestFiles::new(upload_dir, request_id);
    
    let mut stego_image_path: Option<PathBuf> = None;
    
    // Process multipart form data
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => {
                error!("Error getting multipart field: {}", e);
                return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                    request_id,
                    error_codes::VALIDATION_ERROR,
                    &format!("Invalid form data: {}", e)
                )));
            }
        };
        
        // Get field information
        let content_disposition = field.content_disposition();
        let field_name = content_disposition
            .and_then(|cd| cd.get_name())
            .unwrap_or("")
            .to_string();
        
        if field_name == "stego_image" {
            // Get filename
            let filename = content_disposition
                .and_then(|cd| cd.get_filename())
                .unwrap_or("stego_image.png")
                .to_string();
            
            // Create a file to save the uploaded image
            let (path, mut file) = match files.create_file(&filename) {
                Ok((p, f)) => (p, f),
                Err(e) => {
                    error!("Failed to create file: {}", e);
                    return Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                        request_id,
                        error_codes::INTERNAL_ERROR,
                        "Failed to process uploaded file"
                    )));
                }
            };
            
            // Save the file
            let mut size: usize = 0;
            while let Some(chunk) = field.next().await {
                let data = match chunk {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Error reading multipart chunk: {}", e);
                        return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                            request_id,
                            error_codes::VALIDATION_ERROR,
                            &format!("Error reading upload: {}", e)
                        )));
                    }
                };
                
                size += data.len();
                if size > MAX_IMAGE_SIZE {
                    return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                        request_id,
                        error_codes::IMAGE_TOO_LARGE,
                        &format!("Image exceeds maximum size of {} bytes", MAX_IMAGE_SIZE)
                    )));
                }
                
                // Write chunk to file
                if let Err(e) = file.write_all(&data) {
                    error!("Error writing to file: {}", e);
                    return Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                        request_id,
                        error_codes::INTERNAL_ERROR,
                        "Failed to save uploaded file"
                    )));
                }
            }
            
            stego_image_path = Some(path);
        } else {
            // Skip unknown fields
            while let Some(_) = field.next().await {}
        }
    }
    
    // Ensure we have a stego image
    let stego_image_path = match stego_image_path {
        Some(path) => path,
        None => {
            return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                request_id,
                error_codes::VALIDATION_ERROR,
                "Missing stego image"
            )));
        }
    };
    
    // Load the stego image
    let stego_image = match StegoImage::from_file(&stego_image_path) {
        Ok(img) => img,
        Err(e) => {
            error!("Failed to load stego image: {}", e);
            return Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                request_id,
                error_codes::INVALID_IMAGE,
                &format!("Failed to load stego image: {}", e)
            )));
        }
    };
    
    // Create the decoder
    let decoder = create_decoder();
    
    // Decode the message
    let message_bytes = match decoder.decode(&stego_image) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to decode message: {:?}", e);
            return Ok(HttpResponse::BadRequest().json(hide_error_to_response(e, request_id)));
        }
    };
    
    // Check if the message is valid UTF-8
    let text_message = String::from_utf8(message_bytes.clone()).ok();
    
    // Base64 encode the binary message
    let binary_message = BASE64.encode(&message_bytes);
    
    // Create the response
    let response = DecodeResponse {
        request_id,
        status: "success".to_string(),
        message: text_message,
        binary_message,
        message_length: message_bytes.len(),
    };
    
    Ok(HttpResponse::Ok().json(response))
}
