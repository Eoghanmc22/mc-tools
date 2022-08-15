use std::io::{ErrorKind, Read, Write};
use anyhow::{anyhow, Context};
use binary::{slice_serialization, varint};
use binary::slice_serialization::SliceSerializable;
use libdeflater::Decompressor;
use crate::buf::Buffer;
use crate::ctx::{ConnectionContext, GlobalContext};
use crate::error::CommunicationError;
use crate::write;

const PROBE_LEN: usize = 2048;

pub fn read<S, H>(ctx: &mut GlobalContext, connection: &mut ConnectionContext<S>, mut handler: H) -> Result<(), CommunicationError>
where
    S: Read + Write,
    H: FnMut(&RawPacket, &mut Buffer) -> Result<(), CommunicationError>,
{
    let GlobalContext { read_buffer, write_buffer, compression_buffer, decompressor, .. } = ctx;
    let ConnectionContext { compression_threshold, socket, unwritten, unread, writeable, .. } = connection;

    read_buffer.reset();
    write_buffer.reset();

    // Restore any bytes that weren't processed previously
    read_buffer.copy_from(unread.get_written());
    unread.reset();

    while let ReadResult::Read(..) = socket_read(&mut *socket, read_buffer)? {
        while let DecodeResult::Packet(packet, network_len) = decode_packet(read_buffer.get_written(), compression_buffer, decompressor, *compression_threshold)? {
            (handler)(&packet, write_buffer)?;

            read_buffer.consume(network_len);
        }

        // A write per read seems fair
        write::write_buffer(&mut *socket, write_buffer, unwritten, writeable)?;
    }

    // Copy any unprocessed bytes into the `unread` buffer for future processing
    unread.copy_from(read_buffer.get_written());

    Ok(())
}

enum ReadResult {
    Read(usize),
    WouldBlock
}

fn socket_read<S: Read>(mut socket: S, buffer: &mut Buffer) -> Result<ReadResult, CommunicationError> {
    let unwritten = buffer.get_unwritten(PROBE_LEN);

    // Read the stream once
    let read = loop {
        match socket.read(unwritten) {
            Ok(0) => return Err(CommunicationError::Closed),
            Ok(amt) => break amt,
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok(ReadResult::WouldBlock),
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
    Packet(RawPacket<'a>, usize),
    Incomplete
}

#[derive(Debug)]
pub struct RawPacket<'a>(pub &'a [u8]);

const MAXIMUM_PACKET_SIZE: usize = 2097148;

fn decode_packet<'a>(data: &'a [u8], compression_buffer: &'a mut Buffer, decompressor: &mut Decompressor, compression_threshold: i32) -> Result<DecodeResult<'a>, CommunicationError> {
    let available = data.len();

    if available >= 3 {
        let (packet_size, varint_header_bytes) = varint::decode::u21(data).context("packet len read")?;
        let packet_size = packet_size as usize;

        if packet_size > MAXIMUM_PACKET_SIZE {
            return Err(anyhow!("packet size exceeded limit").into());
        }

        let available = available - varint_header_bytes;
        Ok(
            if available >= packet_size {
                let packet = &data[varint_header_bytes..][..packet_size];
                let unframed_packet = if compression_threshold > 0 {
                    handle_compressed_frame(packet, compression_buffer, decompressor, compression_threshold)?
                } else {
                    handle_uncompressed_frame(packet)?
                };

                let network_len = varint_header_bytes + packet_size;
                DecodeResult::Packet(unframed_packet, network_len)
            } else {
                DecodeResult::Incomplete
            }
        )
    } else if available == 2 && data[0] == 1 {
        Ok(
            DecodeResult::Packet(RawPacket(&data[1..2]), 2)
        )
    } else {
        Ok(DecodeResult::Incomplete)
    }
}

// todo Implement this better?

fn handle_uncompressed_frame(data: &[u8]) -> Result<RawPacket, CommunicationError> {
    Ok(RawPacket(data))
}

fn handle_compressed_frame<'a>(mut data: &'a [u8], compression_buffer: &'a mut Buffer, decompressor: &mut Decompressor, compression_threshold: i32) -> Result<RawPacket<'a>, CommunicationError> {
    let data_len = slice_serialization::VarInt::read(&mut data).context("data len read")? as usize;

    // Handle packets too small to be compressed
    if data_len == 0 {
        return Ok(RawPacket(data));
    }

    if data_len > MAXIMUM_PACKET_SIZE {
        return Err(anyhow!("Uncompressed packet size exceeded limit").into());
    }
    if data_len < compression_threshold as usize {
        return Err(anyhow!("Uncompressed packet size is below compression_threshold").into());
    }

    // Decompress the packet
    compression_buffer.reset();
    let unwritten = compression_buffer.get_unwritten(data_len);
    let decompressed = decompressor.zlib_decompress(data, unwritten)?;

    if decompressed != data_len {
        return Err(anyhow!("Decompressed size {decompressed} is not equal to the size received in header {data_len}").into());
    }

    // SAFETY: we just put `decompressed` bytes into the buffer
    unsafe {
        compression_buffer.advance(decompressed);
    }

    Ok(RawPacket(compression_buffer.get_written()))
}
