use std::io;
use libdeflater::{CompressionError, DecompressionError};
use thiserror::Error;

// todo add backtraces when possible
#[derive(Error, Debug)]
pub enum CommunicationError {
    #[error("io with underlying socket failed: {0}")]
    Io(#[from] io::Error),
    #[error("Connection was closed")]
    Closed,
    #[error("Kicked for reason `{0}`")]
    Kicked(String),
    #[error("Got bad data from peer: {0}")]
    BadData(#[from] anyhow::Error),
    #[error("Compression error: {0}")]
    Compression(#[from] CompressionError),
    #[error("Decompression error: {0}")]
    Decompression(#[from] DecompressionError),
}
