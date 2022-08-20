use std::io::{Read, Write};
use libdeflater::{CompressionLvl, Compressor, Decompressor};
use crate::buf::Buffer;

pub mod error;
pub mod buf;
pub mod packet;
pub mod io;

#[derive(Debug)]
pub struct FramedPacket<'a>(pub &'a [u8]);

#[derive(Debug)]
pub struct RawPacket<'a>(pub i32, pub &'a [u8]);

const MAXIMUM_PACKET_SIZE: usize = 2097148;

#[derive(Clone, Debug)]
pub struct ConnectionContext<S: Read + Write> {
    pub compression_threshold: i32,
    pub socket: S,
    // TODO would smallvec or similar be better?
    pub unwritten_buf: Buffer,
    pub unread_buf: Buffer,
    pub writeable: bool
}

impl<S: Read + Write> ConnectionContext<S> {
    pub fn new(socket: S) -> Self {
        Self {
            compression_threshold: -1,
            socket,
            unwritten_buf: Buffer::new(),
            unread_buf: Buffer::new(),
            writeable: false
        }
    }
}

pub struct GlobalContext {
    pub read_buf: Buffer,
    pub write_buf: Buffer,
    pub compression_buf: Buffer,

    pub compressor: Compressor,
    pub decompressor: Decompressor,
}

impl GlobalContext {
    pub fn new() -> Self {
        Self {
            read_buf: Buffer::new(),
            write_buf: Buffer::new(),
            compression_buf: Buffer::new(),
            compressor: Compressor::new(CompressionLvl::fastest()),
            decompressor: Decompressor::new(),
        }
    }
}

impl Default for GlobalContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CompressionContext<'a, 'b, 'c> {
    pub compression_threshold: i32,

    pub compression_buf: &'a mut Buffer,

    pub compressor: &'b mut Compressor,
    pub decompressor: &'c mut Decompressor
}
