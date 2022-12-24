#![feature(split_array)]

use crate::buf::Buffer;
use crate::error::CommunicationError;
use crate::io::{read, write};
use libdeflater::{CompressionLvl, Compressor, Decompressor};
use packet::handle;
use proto::Packet;
use std::fmt::Debug;
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

#[derive(Clone)]
pub struct ConnectionReadContext<D> {
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

impl<D> Debug for ConnectionReadContext<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionReadContext")
            .field("unread_buf", &self.unread_buf)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct ConnectionWriteContext<D> {
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
            socket,
            unwritten_buf: Buffer::new(),
            writeable: true,
        }
    }

    pub fn write_packets<F>(
        &mut self,
        ctx: &mut GlobalWriteContext,
        compression_threshold: i32,
        packets: F,
    ) -> Result<(), CommunicationError>
    where
        F: FnOnce(&mut PacketWriter) -> Result<(), CommunicationError>,
    {
        let (write_buf, compression_ctx) = ctx.compression();

        {
            let mut writer = PacketWriter {
                write_buf,
                compression_ctx,
                compression_threshold,
            };
            (packets)(&mut writer)?;
        }

        self.write_buffer(write_buf)
    }

    pub fn write_packet<'a, P: Packet<'a>>(
        &mut self,
        packet: &'a P,
        ctx: &mut GlobalWriteContext,
        compression_threshold: i32,
    ) -> Result<(), CommunicationError> {
        let (write_buf, mut compression_ctx) = ctx.compression();
        packet::helpers::write_packet(
            packet,
            write_buf,
            &mut compression_ctx,
            compression_threshold,
        )?;
        self.write_buffer(write_buf)
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

impl<D> Debug for ConnectionWriteContext<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionWriteContext")
            .field("unwritten_buf", &self.unwritten_buf)
            .field("writeable", &self.writeable)
            .finish_non_exhaustive()
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

    pub fn decompression(&mut self) -> (&mut Buffer, CompressionReadContext) {
        self.reset();
        (
            &mut self.read_buf,
            CompressionReadContext {
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

impl Debug for GlobalReadContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalReadContext")
            .field("write_buf", &self.read_buf)
            .field("compression_buf", &self.compression_buf)
            .finish_non_exhaustive()
    }
}

pub struct GlobalWriteContext {
    pub write_buf: Buffer,
    pub compression_buf: Buffer,

    pub compressor: Option<Compressor>,
}

impl GlobalWriteContext {
    pub fn new() -> Self {
        Self {
            write_buf: Buffer::new(),
            compression_buf: Buffer::new(),
            compressor: None,
        }
    }

    pub fn compression(&mut self) -> (&mut Buffer, CompressionWriteContext) {
        self.reset();

        let compressor = if let Some(ref mut compressor) = self.compressor {
            compressor
        } else {
            self.compressor = Some(Compressor::new(CompressionLvl::fastest()));
            self.compressor.as_mut().unwrap()
        };

        (
            &mut self.write_buf,
            CompressionWriteContext {
                compression_buf: &mut self.compression_buf,
                compressor,
            },
        )
    }

    pub fn reset(&mut self) {
        self.write_buf.reset();
        self.compression_buf.reset();
    }
}

impl Debug for GlobalWriteContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalWriteContext")
            .field("write_buf", &self.write_buf)
            .field("compression_buf", &self.compression_buf)
            .finish_non_exhaustive()
    }
}

impl Default for GlobalWriteContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CompressionReadContext<'a, 'b> {
    pub compression_buf: &'a mut Buffer,

    pub decompressor: &'b mut Decompressor,
}

impl Debug for CompressionReadContext<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompressionReadContext")
            .field("compression_buf", &self.compression_buf)
            .finish_non_exhaustive()
    }
}

pub struct CompressionWriteContext<'a, 'b> {
    pub compression_buf: &'a mut Buffer,

    pub compressor: &'b mut Compressor,
}

impl Debug for CompressionWriteContext<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompressionWriteContext")
            .field("compression_buf", &self.compression_buf)
            .finish_non_exhaustive()
    }
}

pub struct PacketWriter<'a, 'b, 'c> {
    write_buf: &'a mut Buffer,
    compression_ctx: CompressionWriteContext<'b, 'c>,
    compression_threshold: i32,
}
impl PacketWriter<'_, '_, '_> {
    pub fn write_packet<'a, P: Packet<'a>>(
        &mut self,
        packet: &'a P,
    ) -> Result<(), CommunicationError> {
        packet::helpers::write_packet(
            packet,
            self.write_buf,
            &mut self.compression_ctx,
            self.compression_threshold,
        )?;

        Ok(())
    }
}
