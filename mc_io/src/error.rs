use libdeflater::{CompressionError, DecompressionError};
use proto::DecodingError;
use std::io;
use thiserror::Error;

// todo add backtraces when possible
// todo split into read and write error better
#[derive(Error, Debug)]
pub enum CommunicationError {
    #[error("io with underlying socket failed: {0}")]
    Io(#[from] io::Error),
    #[error("Connection was closed")]
    Closed,
    #[error("Kicked for reason `{0}`")]
    Kicked(String),
    #[error("Write error: {0}")]
    Write(#[from] WriteError),
    #[error("Read error: {0}")]
    Read(#[from] ReadError),
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
    #[error("Packet with size 0 was received")]
    ZeroSizedPacket,
    #[error("Decompression error: {0}")]
    Decompression(#[from] DecompressionError),
    #[error("A received packet was compressed when it shouldn't have been")]
    BadlyCompressed,
    #[error("Received a packet while in an unknown protocol status")]
    BadProtocolState,
    #[error("Received a packet with bad id: {0}")]
    BadPacketID(u8),
    #[error("Received an unparseable packet: {0}")]
    BadPacket(#[from] DecodingError),
}
