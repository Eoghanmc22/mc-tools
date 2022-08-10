use std::io::{ErrorKind, Read, Write};
use std::mem;
use anyhow::{anyhow, Context};
use binary::{slice_serialization, varint};
use binary::slice_serialization::SliceSerializable;
use libdeflater::Decompressor;
use crate::ctx::{ConnectionContext, GlobalContext};
use crate::error::CommunicationError;
use crate::write;

const MIN_PROBE_LEN: usize = 2048;

pub fn read<S, H>(ctx: &mut GlobalContext, connection: &mut ConnectionContext<S>, mut handler: H) -> Result<(), CommunicationError>
where
    S: Read + Write,
    H: for<'a> FnMut(&RawPacket, &'a mut [u8]) -> &'a mut [u8],
{
    let GlobalContext { read_buffer, write_buffer, compression_buffer, decompressor, .. } = ctx;
    let ConnectionContext { compression_threshold, socket, unwritten, unread, writeable, .. } = connection;

    read_buffer.clear();
    write_buffer.clear();

    // SAFETY: this is not really safe, todo: custom buffer impl
    let write_spare_capacity: &mut [u8] = unsafe {
        mem::transmute(write_buffer.spare_capacity_mut())
    };

    read_buffer.extend_from_slice(&unread[..]);

    while let ReadResult::Read(..) = socket_read(&mut *socket, read_buffer)? {
        let mut read = 0;

        while let DecodeResult::Packet(packet) = decode_packet(&read_buffer[read..], compression_buffer, decompressor, *compression_threshold)? {
            let write_spare_capacity_len = write_spare_capacity.len();
            let unused_write_spare_capacity = (handler)(&packet, write_spare_capacity);
            let unused_write_spare_capacity_len = unused_write_spare_capacity.len();

            write::write_slice(socket, &write_spare_capacity[..write_spare_capacity_len - unused_write_spare_capacity_len], unwritten, writeable)?;

            read += packet.0.len();
        }

        read_buffer.drain(..read);
    }

    Ok(())
}

enum ReadResult {
    Read(usize),
    WouldBlock
}

fn socket_read<S: Read>(mut socket: S, buffer: &mut Vec<u8>) -> Result<ReadResult, CommunicationError> {
    // Make sure there is room in the buffer
    buffer.reserve(MIN_PROBE_LEN);

    // SAFETY: this is not really safe, todo: custom buffer impl
    let spare_capacity : &mut [u8] = unsafe {
        mem::transmute(buffer.spare_capacity_mut())
    };

    // Read the stream once
    let read = loop {
        match socket.read(&mut *spare_capacity) {
            Ok(0) => return Err(CommunicationError::Closed),
            Ok(amt) => break amt,
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok(ReadResult::WouldBlock),
            Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(CommunicationError::Io(err)),
        }
    };
    debug_assert!(read <= spare_capacity.len());

    // SAFETY: we just put `read` bytes into the buffer
    unsafe {
        let new_len = buffer.len() + read;
        buffer.set_len(new_len);
    }

    Ok(ReadResult::Read(read))
}

enum DecodeResult<'a> {
    Packet(RawPacket<'a>),
    Incomplete
}

#[derive(Debug)]
pub struct RawPacket<'a>(pub &'a [u8]);

const MAXIMUM_PACKET_SIZE: usize = 2097148;

fn decode_packet<'a>(data: &'a [u8], compression_buffer: &'a mut Vec<u8>, decompressor: &mut Decompressor, compression_threshold: i32) -> Result<DecodeResult<'a>, CommunicationError> {
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

                DecodeResult::Packet(
                    if compression_threshold > 0 {
                        handle_compressed_frame(packet, compression_buffer, decompressor, compression_threshold)?
                    } else {
                        handle_uncompressed_frame(packet)?
                    }
                )
            } else {
                DecodeResult::Incomplete
            }
        )
    } else if available == 2 && data[0] == 1 {
        Ok(
            DecodeResult::Packet(RawPacket(&data[1..2]))
        )
    } else {
        Ok(DecodeResult::Incomplete)
    }
}

fn handle_uncompressed_frame(data: &[u8]) -> Result<RawPacket, CommunicationError> {
    Ok(RawPacket(data))
}

fn handle_compressed_frame<'a>(mut data: &[u8], compression_buffer: &'a mut Vec<u8>, decompressor: &mut Decompressor, compression_threshold: i32) -> Result<RawPacket<'a>, CommunicationError> {
    let data_len = slice_serialization::VarInt::read(&mut data).context("data len read")? as usize;

    if data_len > MAXIMUM_PACKET_SIZE {
        return Err(anyhow!("uncompressed packet size exceeded limit").into());
    }

    compression_buffer.clear();
    compression_buffer.reserve(data_len);

    // SAFETY: this is not really safe, todo: custom buffer impl
    let spare_capacity = unsafe {
        mem::transmute(compression_buffer.spare_capacity_mut())
    };

    let decompressed = decompressor.zlib_decompress(data, spare_capacity)?;
    debug_assert!(decompressed == data_len);
    debug_assert!(decompressed >= compression_threshold as usize);

    // SAFETY: we just put `decompressed` bytes into the buffer
    unsafe {
        let new_len = compression_buffer.len() + decompressed;
        compression_buffer.set_len(new_len);
    }

    Ok(RawPacket(&compression_buffer[..decompressed]))
}
