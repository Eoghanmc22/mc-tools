use libdeflater::{CompressionLvl, Compressor, Decompressor};
use std::io::{Read, Write};

const GLOBAL_BUFFER_CAP: usize = 4096;
const BOT_BUFFER_CAP: usize = 1024;

pub struct ConnectionContext<S: Read + Write> {
    pub(crate) compression_threshold: i32,
    pub(crate) socket: S,
    // TODO would smallvec or similar be better?
    pub(crate) unwritten: Vec<u8>,
    pub(crate) unread: Vec<u8>,
    pub(crate) writeable: bool
}

impl<S: Read + Write> ConnectionContext<S> {
    pub fn new(socket: S) -> Self {
        Self {
            compression_threshold: -1,
            socket,
            unwritten: Vec::with_capacity(BOT_BUFFER_CAP),
            unread: Vec::with_capacity(BOT_BUFFER_CAP),
            writeable: false
        }
    }
}

pub struct GlobalContext {
    pub(crate) read_buffer: Vec<u8>,
    pub(crate) write_buffer: Vec<u8>,
    pub(crate) compression_buffer: Vec<u8>,

    pub(crate) compressor: Compressor,
    pub(crate) decompressor: Decompressor,
}

impl GlobalContext {
    pub fn new() -> Self {
        Self {
            read_buffer: Vec::with_capacity(GLOBAL_BUFFER_CAP),
            write_buffer: Vec::with_capacity(GLOBAL_BUFFER_CAP),
            compression_buffer: Vec::with_capacity(GLOBAL_BUFFER_CAP),
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
