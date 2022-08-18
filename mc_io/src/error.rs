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
    #[error("Write error: {0}")]
    Write(#[from] WriteError),
    #[error("Read error: {0}")]
    Read(#[from] ReadError)
}

#[derive(Error, Debug)]
pub enum WriteError {
    #[error("Packet size exceeded limit")]
    PacketTooLarge,
    #[error("Compression error: {0}")]
    Compression(#[from] CompressionError),
}

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Packet size exceeded limit")]
    PacketTooLarge,
    #[error("Decompression error: {0}")]
    Decompression(#[from] DecompressionError),
    #[error("Error reading varint")]
    VarInt,
    #[error("A reveived packet was compressed when it shouldn't have been")]
    BadlyCompressed
}

