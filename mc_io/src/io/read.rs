use proto::primitive::V21;
use proto::Data;

use crate::buf::Buffer;
use crate::error::{CommunicationError, ReadError};
use crate::{
    CompressionReadContext, ConnectionReadContext, FramedPacket, GlobalReadContext,
    MAXIMUM_PACKET_SIZE,
};
use std::io::{ErrorKind, Read, Write};

const PROBE_LEN: usize = 2048;

pub trait ReadHandler {
    fn handle(
        &mut self,
        packet: &FramedPacket,
        compression_ctx: CompressionReadContext,
    ) -> Result<(), CommunicationError>;
}

impl<F> ReadHandler for F
where
    F: FnMut(&FramedPacket, CompressionReadContext) -> Result<(), CommunicationError>,
{
    fn handle(
        &mut self,
        packet: &FramedPacket,
        compression_ctx: CompressionReadContext,
    ) -> Result<(), CommunicationError> {
        (self)(packet, compression_ctx)
    }
}

pub fn read<S>(
    ctx: &mut GlobalReadContext,
    connection: &mut ConnectionReadContext<S>,
    mut handler: impl ReadHandler,
) -> Result<(), CommunicationError>
where
    S: Read + Write,
{
    let GlobalReadContext {
        read_buf,
        compression_buf,
        decompressor,
    } = ctx;
    let ConnectionReadContext {
        compression_threshold,
        socket,
        unread_buf,
    } = connection;

    read_buf.reset();

    // Restore any bytes that weren't processed previously
    read_buf.copy_from(unread_buf.get_written());
    unread_buf.reset();

    while let ReadResult::Read(..) = socket_read(&mut *socket, read_buf)? {
        compression_buf.reset();

        while let DecodeResult::Packet(packet, network_len) = next_packet(read_buf.get_written())? {
            let compression_ctx = CompressionReadContext {
                compression_threshold: *compression_threshold,
                compression_buf,
                decompressor,
            };

            handler.handle(&packet, compression_ctx)?;

            read_buf.consume(network_len);
        }
    }

    // Copy any unprocessed bytes into the `unread` buffer for future processing
    unread_buf.copy_from(read_buf.get_written());

    Ok(())
}

enum ReadResult {
    Read(usize),
    WouldBlock,
}

fn socket_read<S: Read>(
    mut socket: S,
    buffer: &mut Buffer,
) -> Result<ReadResult, CommunicationError> {
    let unwritten = buffer.get_unwritten(PROBE_LEN);

    // Read the stream once
    let read = loop {
        match socket.read(unwritten) {
            Ok(0) => return Err(CommunicationError::Closed),
            Ok(amt) => break amt,
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => {
                return Ok(ReadResult::WouldBlock)
            }
            Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(CommunicationError::Io(err)),
        }
    };

    // SAFETY: We just put `read` bytes into the buffer
    unsafe {
        buffer.advance(read);
    }

    Ok(ReadResult::Read(read))
}

enum DecodeResult<'a> {
    Packet(FramedPacket<'a>, usize),
    Incomplete,
}

fn next_packet(mut data: &[u8]) -> Result<DecodeResult, CommunicationError> {
    let available = data.len();

    if available >= 3 {
        let packet_size = V21::try_decode(&mut data)
            .map_err(|err| CommunicationError::from(ReadError::from(err)))?
            .into();
        let varint_size = available - data.len();

        if packet_size > MAXIMUM_PACKET_SIZE {
            return Err(ReadError::PacketTooLarge.into());
        }

        if data.len() >= packet_size {
            Ok(DecodeResult::Packet(
                FramedPacket(&data[..packet_size]),
                varint_size + packet_size,
            ))
        } else {
            Ok(DecodeResult::Incomplete)
        }
    } else if available == 2 && data[0] == 1 {
        Ok(DecodeResult::Packet(FramedPacket(&data[1..2]), 2))
    } else if available == 1 && data[0] == 0 {
        Err(ReadError::ZeroSizedPacket.into())
    } else {
        Ok(DecodeResult::Incomplete)
    }
}
