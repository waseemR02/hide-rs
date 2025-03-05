//! REST API server for hide-rs steganography library

use actix_files::Files;
use actix_web::{middleware::Logger, App, HttpServer};
use anyhow::Result;
use dotenv::dotenv;
use hide_rs::api::{
    handlers::{AppState, ServerConfig},
    routes::configure_routes,
};
use log::{error, info};
use std::io;

// Remove the ServerConfig definition since we're now using the one from handlers

/// Load configuration from environment or file
fn load_config() -> Result<ServerConfig> {
    // First try to load .env file if present
    let _ = dotenv();

    // Create default config
    let mut config = ServerConfig::default();

    // Override with environment variables if present
    if let Ok(host) = std::env::var("HIDE_HOST") {
        config.host = host;
    }

    if let Ok(port_str) = std::env::var("HIDE_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            config.port = port;
        }
    }

    if let Ok(upload_dir) = std::env::var("HIDE_UPLOAD_DIR") {
        config.upload_dir = upload_dir;
    }

    // Create upload directory if it doesn't exist
    let upload_dir = std::path::Path::new(&config.upload_dir);
    if !upload_dir.exists() {
        std::fs::create_dir_all(upload_dir)?;
    }

    Ok(config)
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Load configuration
    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load configuration: {}", err);
            return Err(io::Error::new(io::ErrorKind::Other, "Configuration error"));
        }
    };

    // Create application state
    let state = actix_web::web::Data::new(AppState {
        config: config.clone(),
    });

    // Start server
    info!("Starting server at http://{}:{}", config.host, config.port);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(state.clone())
            .configure(configure_routes)
            .service(Files::new("/static", "./static").show_files_listing())
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use hide_rs::api::routes::{health_check, ping};

    #[actix_web::test]
    async fn test_health_check() {
        // Create test application
        let app =
            test::init_service(App::new().route("/api/health", web::get().to(health_check))).await;

        // Send request
        let req = test::TestRequest::get().uri("/api/health").to_request();
        let resp = test::call_service(&app, req).await;

        // Check response
        assert!(resp.status().is_success());

        // Parse response body
        let body = test::read_body(resp).await;
        let response: HealthResponse = serde_json::from_slice(&body).unwrap();

        // Verify response content
        assert_eq!(response.status, "ok");
        assert_eq!(response.version, hide_rs::VERSION);
    }

    #[actix_web::test]
    async fn test_ping() {
        // Create test application
        let app = test::init_service(App::new().route("/api/ping", web::get().to(ping))).await;

        // Send request
        let req = test::TestRequest::get().uri("/api/ping").to_request();
        let resp = test::call_service(&app, req).await;

        // Check response
        assert!(resp.status().is_success());

        // Verify response content
        let body = test::read_body(resp).await;
        assert_eq!(body, "pong");
    }
}
