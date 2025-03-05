//! API route definitions for the REST API

use crate::api::handlers::*;
use actix_web::{web, HttpResponse, Responder};
use actix_multipart::Multipart;
use std::path::Path;

/// Health check endpoint
pub async fn health_check() -> impl Responder {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: crate::VERSION.to_string(),
    };
    HttpResponse::Ok().json(response)
}

/// Ping endpoint (simple test)
pub async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

/// Encode message endpoint 
/// This endpoint handles steganography encoding
pub async fn encode(
    payload: Multipart,
    data: web::Data<AppState>,
) -> impl Responder {
    // Convert String to &Path
    let upload_dir = Path::new(&data.config.upload_dir);
    process_encode_form(payload, upload_dir).await
}

/// Decode message endpoint
/// This endpoint handles steganography decoding
pub async fn decode(
    payload: Multipart,
    data: web::Data<AppState>,
) -> impl Responder {
    // Convert String to &Path
    let upload_dir = Path::new(&data.config.upload_dir);
    process_decode_form(payload, upload_dir).await
}

/// Get encoded image endpoint
pub async fn get_image(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    // Convert String to &Path
    let upload_dir = Path::new(&data.config.upload_dir);
    serve_encoded_image(path.into_inner(), upload_dir).await
}

/// Configure all API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health_check))
            .route("/ping", web::get().to(ping))
            .route("/encode", web::post().to(encode))
            .route("/decode", web::post().to(decode))
            .route("/images/{image_id}", web::get().to(get_image))
    );
}
