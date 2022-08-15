use libdeflater::{CompressionLvl, Compressor, Decompressor};
use std::io::{Read, Write};
use crate::buf::Buffer;

pub struct ConnectionContext<S: Read + Write> {
    pub(crate) compression_threshold: i32,
    pub(crate) socket: S,
    // TODO would smallvec or similar be better?
    pub(crate) unwritten: Buffer,
    pub(crate) unread: Buffer,
    pub(crate) writeable: bool
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
    pub(crate) read_buffer: Buffer,
    pub(crate) write_buffer: Buffer,
    pub(crate) compression_buffer: Buffer,

    pub(crate) compressor: Compressor,
    pub(crate) decompressor: Decompressor,
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
