use actix_web::{test, web, App};
use hide_rs::api::{handlers::AppState, routes::configure_routes};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tempfile::tempdir;

#[actix_web::test]
async fn test_encode_endpoint() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let upload_dir = temp_dir.path().to_path_buf();

    // Create a test image
    let test_image_path = upload_dir.join("test_image.png");
    create_test_image(&test_image_path, 100, 100);

    // Create application state
    let state = web::Data::new(AppState {
        config: hide_rs::api::handlers::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            upload_dir: upload_dir.to_string_lossy().to_string(),
        },
    });

    // Create test application
    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    // Create a multipart form with an image and message
    let (payload, content_type) = create_test_multipart(&test_image_path, "This is a test message");

    // Send request to the endpoint
    let req = test::TestRequest::post()
        .uri("/api/encode")
        .insert_header(("content-type", content_type))
        .set_payload(payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Check response status
    assert!(
        resp.status().is_success(),
        "Response status is not success: {}",
        resp.status()
    );

    // Parse response body
    let body = test::read_body(resp).await;
    let json_response: serde_json::Value =
        serde_json::from_slice(&body).expect("Failed to parse JSON response");

    // Check that the response contains the expected fields
    assert!(
        json_response.get("status").is_some(),
        "Missing status field"
    );
    assert_eq!(json_response["status"], "success", "Status is not success");
    assert!(
        json_response.get("image_id").is_some(),
        "Missing image_id field"
    );
    assert!(
        json_response.get("download_url").is_some(),
        "Missing download_url field"
    );
}

#[actix_web::test]
async fn test_encode_empty_message() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let upload_dir = temp_dir.path().to_path_buf();

    // Create a test image
    let test_image_path = upload_dir.join("test_image.png");
    create_test_image(&test_image_path, 100, 100);

    // Create application state
    let state = web::Data::new(AppState {
        config: hide_rs::api::handlers::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            upload_dir: upload_dir.to_string_lossy().to_string(),
        },
    });

    // Create test application
    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    // Create a multipart form with an image and an empty message
    let (payload, content_type) = create_test_multipart(&test_image_path, "");

    // Send request to the endpoint
    let req = test::TestRequest::post()
        .uri("/api/encode")
        .insert_header(("content-type", content_type))
        .set_payload(payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Check response status
    assert!(
        resp.status().is_success(),
        "Response status is not success: {}",
        resp.status()
    );
}

#[actix_web::test]
async fn test_encode_no_image() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let upload_dir = temp_dir.path().to_path_buf();

    // Create application state
    let state = web::Data::new(AppState {
        config: hide_rs::api::handlers::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            upload_dir: upload_dir.to_string_lossy().to_string(),
        },
    });

    // Create test application
    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    // Create a multipart form with just a message, no image
    let boundary = "------------------------abcdef1234567890";
    let content_type = format!("multipart/form-data; boundary={}", boundary);

    let payload = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"message\"\r\n\r\n\
         This is a test message\r\n\
         --{boundary}--\r\n",
        boundary = boundary
    );

    // Send request to the endpoint
    let req = test::TestRequest::post()
        .uri("/api/encode")
        .insert_header(("content-type", content_type))
        .set_payload(payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should fail with bad request
    assert_eq!(resp.status(), 400);

    // Parse response body
    let body = test::read_body(resp).await;
    let json_response: serde_json::Value =
        serde_json::from_slice(&body).expect("Failed to parse JSON response");

    // Check error details
    assert_eq!(json_response["status"], "error");
    assert_eq!(json_response["error_code"], "validation_error");
    assert!(json_response["message"]
        .as_str()
        .unwrap()
        .contains("Missing cover image"));
}

#[actix_web::test]
async fn test_encode_no_message() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let upload_dir = temp_dir.path().to_path_buf();

    // Create a test image
    let test_image_path = upload_dir.join("test_image.png");
    create_test_image(&test_image_path, 100, 100);

    // Create application state
    let state = web::Data::new(AppState {
        config: hide_rs::api::handlers::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            upload_dir: upload_dir.to_string_lossy().to_string(),
        },
    });

    // Create test application
    let app = test::init_service(App::new().app_data(state).configure(configure_routes)).await;

    // Create a multipart form with just an image, no message
    let boundary = "------------------------abcdef1234567890";
    let content_type = format!("multipart/form-data; boundary={}", boundary);

    // Read the test image file
    let mut file_data = Vec::new();
    let mut file = File::open(&test_image_path).unwrap();
    file.read_to_end(&mut file_data).unwrap();

    let payload = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"cover_image\"; filename=\"test_image.png\"\r\n\
         Content-Type: image/png\r\n\r\n",
        boundary = boundary
    );

    // Combine the text part with the binary file data
    let mut body = Vec::new();
    body.extend_from_slice(payload.as_bytes());
    body.extend_from_slice(&file_data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n", boundary = boundary).as_bytes());

    // Send request to the endpoint
    let req = test::TestRequest::post()
        .uri("/api/encode")
        .insert_header(("content-type", content_type))
        .set_payload(body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should fail with bad request
    assert_eq!(resp.status(), 400);

    // Parse response body
    let body = test::read_body(resp).await;
    let json_response: serde_json::Value =
        serde_json::from_slice(&body).expect("Failed to parse JSON response");

    // Check error details
    assert_eq!(json_response["status"], "error");
    assert_eq!(json_response["error_code"], "validation_error");
    assert!(json_response["message"]
        .as_str()
        .unwrap()
        .contains("Missing message"));
}

// Helper to create a test image
fn create_test_image(path: &PathBuf, width: u32, height: u32) {
    let img = image::RgbImage::new(width, height);
    img.save(path).unwrap();
}

// Helper to create a multipart form with an image and message
fn create_test_multipart(image_path: &PathBuf, message: &str) -> (Vec<u8>, String) {
    let boundary = "------------------------abcdef1234567890";
    let content_type = format!("multipart/form-data; boundary={}", boundary);

    // Read the test image file
    let mut file_data = Vec::new();
    let mut file = File::open(image_path).unwrap();
    file.read_to_end(&mut file_data).unwrap();

    let payload = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"cover_image\"; filename=\"test_image.png\"\r\n\
         Content-Type: image/png\r\n\r\n",
        boundary = boundary
    );

    let message_part = format!(
        "\r\n--{boundary}\r\n\
         Content-Disposition: form-data; name=\"message\"\r\n\r\n\
         {message}\r\n\
         --{boundary}--\r\n",
        boundary = boundary,
        message = message
    );

    // Combine the parts with the binary file data
    let mut body = Vec::new();
    body.extend_from_slice(payload.as_bytes());
    body.extend_from_slice(&file_data);
    body.extend_from_slice(message_part.as_bytes());

    (body, content_type)
}
