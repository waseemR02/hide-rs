//! Command-line interface for hide-rs steganography library

use clap::{Parser, Subcommand};
use hide_rs::decoder::create_decoder;
use hide_rs::encoder::create_encoder;
use hide_rs::raw_decoder;
use std::fs;
use std::path::PathBuf;

/// Command-line arguments
#[derive(Parser)]
#[command(author, version, about = "Hide messages in images using steganography", long_about = None)]
struct Cli {
    /// Operation mode
    #[command(subcommand)]
    command: Commands,
}

/// Supported commands
#[derive(Subcommand)]
enum Commands {
    /// Hide a message in an image
    Encode {
        /// Path to the cover image file
        #[arg(short, long)]
        image: PathBuf,

        /// The message to hide (use quotes for multiple words)
        #[arg(short, long)]
        message: String,

        /// Path to save the output stego image
        #[arg(short, long)]
        output: PathBuf,

        /// Read message from file instead of command line
        #[arg(short = 'f', long)]
        file: Option<PathBuf>,
    },
    /// Extract a hidden message from an image
    Decode {
        /// Path to the stego image file
        #[arg(short, long)]
        image: PathBuf,

        /// Display output as hexadecimal for binary data
        #[arg(short, long)]
        hex: bool,

        /// Extract raw data regardless of header format validity
        #[arg(short, long, help = "Extract raw data without header validation")]
        raw: bool,

        /// Save output to file instead of displaying
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Encode {
            image,
            message,
            output,
            file,
        } => {
            encode_message(image, message, output, file);
        }
        Commands::Decode {
            image,
            hex,
            raw,
            output,
        } => {
            decode_message(image, *hex, *raw, output);
        }
    }
}

/// Encode a message into an image
fn encode_message(
    image_path: &PathBuf,
    message_text: &str,
    output_path: &PathBuf,
    message_file: &Option<PathBuf>,
) {
    // Determine the message source and read it
    let message = if let Some(file_path) = message_file {
        // Read from file
        fs::read(file_path)
            .unwrap_or_else(|_| panic!("Failed to read message file: {}", file_path.display()))
    } else {
        // Use the message from the command line
        message_text.as_bytes().to_vec()
    };

    println!("Message size: {} bytes", message.len());

    // Create encoder
    let encoder = create_encoder();

    // Encode the message
    println!("Encoding message into image: {}", image_path.display());
    encoder
        .encode_file(image_path, &message, output_path)
        .expect("Failed to encode message");
    println!("Message successfully hidden in: {}", output_path.display());
}

/// Decode a message from an image and display it in the console
fn decode_message(
    image_path: &PathBuf,
    show_hex: bool,
    raw_mode: bool,
    output_file: &Option<PathBuf>,
) {
    println!("Extracting hidden message from: {}", image_path.display());

    // Load the stego image
    let stego_image =
        hide_rs::img::StegoImage::from_file(image_path).expect("Failed to load image");

    let decoded_message = if raw_mode {
        // Use raw decoder to extract all data without header validation
        println!("Using raw extraction mode (ignoring header format)");
        raw_decoder::extract_raw_data(&stego_image).expect("Failed to extract raw data")
    } else {
        // Use standard decoder
        let decoder = create_decoder();
        decoder
            .decode(&stego_image)
            .expect("Failed to decode message")
    };

    println!("Message size: {} bytes", decoded_message.len());

    // Save to file if output was specified
    if let Some(output_path) = output_file {
        fs::write(output_path, &decoded_message).expect("Failed to write output file");
        println!("Output written to: {}", output_path.display());

        if raw_mode {
            // Also generate and display a data preview
            let preview = raw_decoder::format_data_preview(&decoded_message, 32);
            println!("\n{}", preview);
        }

        return; // Don't display content when saving to file
    }

    // Display content according to mode and type
    if raw_mode {
        // In raw mode, always show data analysis
        let preview = raw_decoder::format_data_preview(&decoded_message, 32);
        println!("\n{}", preview);
    } else {
        // In normal mode, try to display as text if possible
        match std::str::from_utf8(&decoded_message) {
            Ok(message_str) if !show_hex => {
                // Message is valid UTF-8 and we're not forcing hex display
                println!("\n----- DECODED MESSAGE -----");
                println!("{}", message_str);
                println!("-------------------------\n");
            }
            _ => {
                // Message is binary or we want hex display
                println!("\n----- BINARY MESSAGE (hex) -----");
                for (i, byte) in decoded_message.iter().enumerate() {
                    print!("{:02x} ", byte);
                    // Add line break every 16 bytes for readability
                    if (i + 1) % 16 == 0 {
                        println!();
                    }
                }
                // Ensure we end with a newline
                if decoded_message.len() % 16 != 0 {
                    println!();
                }
                println!("--------------------------------\n");
            }
        }
    }
}
