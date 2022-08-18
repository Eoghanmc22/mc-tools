use libdeflater::{CompressionLvl, Compressor, Decompressor};
use std::io::{Read, Write};
use crate::buf::Buffer;

pub struct ConnectionContext<S: Read + Write> {
    pub compression_threshold: i32,
    pub socket: S,
    // TODO would smallvec or similar be better?
    pub unwritten: Buffer,
    pub unread: Buffer,
    pub writeable: bool
}

impl<S: Read + Write> ConnectionContext<S> {
    pub fn new(socket: S) -> Self {
        Self {
            compression_threshold: -1,
            socket,
            unwritten: Buffer::new(),
            unread: Buffer::new(),
            writeable: false
        }
    }
}

pub struct GlobalContext {
    pub read_buffer: Buffer,
    pub write_buffer: Buffer,
    pub compression_buffer: Buffer,

    pub compressor: Compressor,
    pub decompressor: Decompressor,
}

impl GlobalContext {
    pub fn new() -> Self {
        Self {
            read_buffer: Buffer::new(),
            write_buffer: Buffer::new(),
            compression_buffer: Buffer::new(),
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
