#![feature(split_array)]

use crate::buf::Buffer;
use crate::error::CommunicationError;
use crate::io::{read, write};
use libdeflater::{CompressionLvl, Compressor, Decompressor};
use packet::handle;
use proto::Packet;
use std::io::{Read, Write};
use std::ops::Deref;

// Re-exports
pub use packet::handle::PacketHandler;

pub mod buf;
pub mod error;
pub mod io;
pub mod packet;

#[derive(Debug)]
pub struct FramedPacket<'a>(pub &'a [u8]);

#[derive(Debug)]
pub struct RawPacket<'a>(pub &'a [u8]);
impl<'a> From<RawPacket<'a>> for &'a [u8] {
    fn from(value: RawPacket<'a>) -> Self {
        value.0
    }
}

const MAXIMUM_PACKET_SIZE: usize = 2097148;

#[derive(Clone, Debug)]
pub struct ConnectionReadContext<D> {
    pub compression_threshold: i32,
    pub socket: D,
    // TODO would smallvec or similar be better?
    pub unread_buf: Buffer,
}

impl<D, S> ConnectionReadContext<D>
where
    D: Deref<Target = S>,
    for<'a> &'a S: Read,
{
    pub fn new(socket: D) -> Self {
        Self {
            compression_threshold: -1,
            socket,
            unread_buf: Buffer::new(),
        }
    }

    pub fn read_packets<H: PacketHandler>(
        &mut self,
        ctx: &mut GlobalReadContext,
        handler: &mut H,
    ) -> Result<(), CommunicationError> {
        let handler = handle::create_handler(handler);
        self.read(ctx, handler)
    }

    pub fn read<F>(
        &mut self,
        ctx: &mut GlobalReadContext,
        handler: F,
    ) -> Result<(), CommunicationError>
    where
        F: FnMut(&FramedPacket, CompressionReadContext) -> Result<(), CommunicationError>,
    {
        read::read(ctx, self, handler)
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionWriteContext<D> {
    pub compression_threshold: i32,
    pub socket: D,
    // TODO would smallvec or similar be better?
    pub unwritten_buf: Buffer,
    pub writeable: bool,
}

impl<D, S> ConnectionWriteContext<D>
where
    D: Deref<Target = S>,
    for<'a> &'a S: Write,
{
    pub fn new(socket: D) -> Self {
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
        let (write_buf, compression_ctx) = ctx.compression(self.compression_threshold);
        packet::helpers::write_packet(packet, write_buf, compression_ctx)?;
        Ok(self.write_buffer(write_buf)?)
    }

    pub fn write_buffer(&mut self, to_write: &mut Buffer) -> Result<(), CommunicationError> {
        self.write_slice(to_write.get_written())
    }

    pub fn write_slice(&mut self, to_write: &[u8]) -> Result<(), CommunicationError> {
        write::write_slice(
            D::deref(&self.socket),
            to_write,
            &mut self.unwritten_buf,
            &mut self.writeable,
        )
    }

    pub fn write_unwritten(&mut self) -> Result<(), CommunicationError> {
        write::write_unwritten(self)
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
