#![feature(split_array)]

use crate::buf::Buffer;
use crate::error::CommunicationError;
use crate::io::write;
use libdeflater::{CompressionLvl, Compressor, Decompressor};
use proto::Packet;
use std::io::{Read, Write};

pub mod buf;
pub mod error;
pub mod io;
pub mod packet;

#[derive(Debug)]
pub struct FramedPacket<'a>(pub &'a [u8]);

#[derive(Debug)]
pub struct RawPacket<'a>(pub u8, pub &'a [u8]);

const MAXIMUM_PACKET_SIZE: usize = 2097148;

#[derive(Clone, Debug)]
pub struct ConnectionReadContext<S: Read> {
    pub compression_threshold: i32,
    pub socket: S,
    // TODO would smallvec or similar be better?
    pub unread_buf: Buffer,
}

impl<S: Read> ConnectionReadContext<S> {
    pub fn new(socket: S) -> Self {
        Self {
            compression_threshold: -1,
            socket,
            unread_buf: Buffer::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionWriteContext<S: Write> {
    pub compression_threshold: i32,
    pub socket: S,
    // TODO would smallvec or similar be better?
    pub unwritten_buf: Buffer,
    pub writeable: bool,
}

impl<S: Write> ConnectionWriteContext<S> {
    pub fn new(socket: S) -> Self {
        Self {
            compression_threshold: -1,
            socket,
            unwritten_buf: Buffer::new(),
            writeable: false,
        }
    }

    pub fn write_packet<'a, P: Packet<'a>>(
        &mut self,
        packet: &'a P,
        ctx: &mut GlobalWriteContext,
    ) -> Result<(), CommunicationError> {
        ctx.reset();
        let (write_buf, compression_ctx) = ctx.compression(self.compression_threshold);
        packet::helpers::write_packet(packet, write_buf, compression_ctx)?;
        Ok(self.write_buffer(write_buf)?)
    }

    pub fn write_buffer(&mut self, to_write: &mut Buffer) -> Result<(), CommunicationError> {
        write::write_buffer(
            &mut self.socket,
            to_write,
            &mut self.unwritten_buf,
            &mut self.writeable,
        )
    }

    pub fn write_slice(&mut self, to_write: &[u8]) -> Result<(), CommunicationError> {
        write::write_slice(
            &mut self.socket,
            to_write,
            &mut self.unwritten_buf,
            &mut self.writeable,
        )
    }
}

pub struct GlobalReadContext {
    pub read_buf: Buffer,
    pub compression_buf: Buffer,

    pub decompressor: Decompressor,
}

impl GlobalReadContext {
    pub fn new() -> Self {
        Self {
            read_buf: Buffer::new(),
            compression_buf: Buffer::new(),
            decompressor: Decompressor::new(),
        }
    }

    pub fn decompression(
        &mut self,
        compression_threshold: i32,
    ) -> (&mut Buffer, CompressionReadContext) {
        self.reset();
        (
            &mut self.read_buf,
            CompressionReadContext {
                compression_threshold,
                compression_buf: &mut self.compression_buf,
                decompressor: &mut self.decompressor,
            },
        )
    }

    pub fn reset(&mut self) {
        self.read_buf.reset();
        self.compression_buf.reset();
    }
}

impl Default for GlobalReadContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GlobalWriteContext {
    pub write_buf: Buffer,
    pub compression_buf: Buffer,

    pub compressor: Compressor,
}

impl GlobalWriteContext {
    pub fn new() -> Self {
        Self {
            write_buf: Buffer::new(),
            compression_buf: Buffer::new(),
            compressor: Compressor::new(CompressionLvl::fastest()),
        }
    }

    pub fn compression(
        &mut self,
        compression_threshold: i32,
    ) -> (&mut Buffer, CompressionWriteContext) {
        self.reset();
        (
            &mut self.write_buf,
            CompressionWriteContext {
                compression_threshold,
                compression_buf: &mut self.compression_buf,
                compressor: &mut self.compressor,
            },
        )
    }

    pub fn reset(&mut self) {
        self.write_buf.reset();
        self.compression_buf.reset();
    }
}

impl Default for GlobalWriteContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CompressionReadContext<'a, 'b> {
    pub compression_threshold: i32,

    pub compression_buf: &'a mut Buffer,

    pub decompressor: &'b mut Decompressor,
}

pub struct CompressionWriteContext<'a, 'b> {
    pub compression_threshold: i32,

    pub compression_buf: &'a mut Buffer,

    pub compressor: &'b mut Compressor,
}
