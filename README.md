# hide-rs

A steganography library and tools using the Binary Lower Triangular Matrix (BLTM) method for hiding messages in images.

## Features

- Hide text or binary data in images with minimal visual changes
- Advanced BLTM steganography algorithm for secure message embedding
- Command-line interface for easy encoding and decoding
- REST API server for web-based steganography operations
- Support for various image formats (PNG, JPEG, BMP, etc.)

## Installation

### From crates.io (not yet published)
```bash
cargo install hide-rs
```

### From source
```bash
git clone https://github.com/waseemr02/hide-rs.git
cd hide-rs
cargo install --path .
```

## CLI Usage

The `hide` command-line tool provides easy access to steganography operations.

### Hiding data in an image

```bash
# Hide text in an image
hide encode --image cover.png --message "Secret message" --output stego.png

# Hide data from a file
hide encode --image cover.png --file secret.txt --output stego.png
```

### Extracting data from an image

```bash
# Extract hidden message (automatically detects text vs binary)
hide decode --image stego.png

# Extract binary data and show in hexadecimal format
hide decode --image stego.png --hex
```

### CLI Options

```
Commands:
  encode    Hide a message in an image
  decode    Extract a hidden message from an image
  help      Print help information
```

## Server Usage

### Starting the server

```bash
# Start server on default port (from config)
hide-server

# Configure using environment variables
HIDE_HOST=127.0.0.1 HIDE_PORT=3000 HIDE_UPLOAD_DIR=/tmp/uploads hide-server
```

### Environment Variables

- `HIDE_HOST`: Host address to bind (default: "127.0.0.1")
- `HIDE_PORT`: Port number to listen on (default: 8080)
- `HIDE_UPLOAD_DIR`: Directory for temporary file uploads (default: "./uploads")

### API Endpoints

#### Health Check
```
GET /api/health
```

Response:
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

#### Quick Check
```
GET /api/ping
```

Response: `pong`

## License

This project is licensed under the MIT License - see the LICENSE file for details.
